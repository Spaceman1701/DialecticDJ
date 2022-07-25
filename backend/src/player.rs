use std::{
    collections::VecDeque,
    process::id,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Error, Result};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{AdditionalType, CurrentPlaybackContext, Device, PlayableItem, TrackId},
    AuthCodeSpotify,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use crate::{
    authentication::{AuthenticationState, ManagedAuthState, SpotifyClient},
    model::TrackInfo,
};

pub type PlayerCommandQueue = Sender<PlayerCommand>;

pub struct PlayerCommader {
    sender: PlayerCommandQueue,
}

impl PlayerCommader {
    fn new(sender: PlayerCommandQueue) -> PlayerCommader {
        return PlayerCommader { sender: sender };
    }

    pub async fn get_currently_playing_track(&self) -> Result<Option<TrackInfo>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender.send(PlayerCommand::GetCurrentTrack(tx)).await?;

        return Ok(rx.await.unwrap());
    }

    pub async fn add_track_to_queue(&self, track_id: TrackId) -> Result<()> {
        self.sender.send(PlayerCommand::AddTrack(track_id)).await?;
        Ok(())
    }

    pub async fn get_queued_tracks(&self) -> Result<Vec<TrackInfo>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender.send(PlayerCommand::GetTrackQueue(tx)).await?;
        Ok(rx.await.unwrap())
    }

    pub async fn start(&self) -> Result<()> {
        self.sender.send(PlayerCommand::Start).await?;
        Ok(())
    }
}

struct PlayerState {
    auth_state: ManagedAuthState,
    queue: VecDeque<TrackInfo>,
    currently_playing: Option<InProgressTrack>,
    cmd_rx: Receiver<PlayerCommand>,
    cmd_tx: PlayerCommandQueue,
    target_device: Option<Device>,
}

#[derive(Debug)]
pub enum PlayerCommand {
    ///Start the player from pause
    Start,

    ///Wake and configure the next track. Sent on a timer
    Wake,

    ///Add a track suggestion to the queue
    AddTrack(TrackId),

    ///Return the currently playing track using the sender
    GetCurrentTrack(oneshot::Sender<Option<TrackInfo>>),

    ///Return the queue of tracks
    GetTrackQueue(oneshot::Sender<Vec<TrackInfo>>),
}

impl PlayerState {
    fn new(auth_state: ManagedAuthState) -> (PlayerState, PlayerCommandQueue) {
        let (tx, rx) = tokio::sync::mpsc::channel(64); //TODO: consider unbounded channel here
        (
            PlayerState {
                auth_state: auth_state,
                queue: VecDeque::new(),
                currently_playing: None,
                cmd_rx: rx,
                cmd_tx: tx.clone(),
                target_device: None,
            },
            tx,
        )
    }

    async fn spotify(&self) -> Option<AuthCodeSpotify> {
        let mut auth_value = self.auth_state.lock().await;
        if auth_value.is_none() {
            return None;
        } else {
            let a = auth_value.as_mut().unwrap();
            return Some(a.client().await);
        }
    }

    async fn find_target_device(&mut self) -> Result<()> {
        if let Some(spotify) = self.spotify().await {
            let playback = spotify
                .current_playback(
                    None,
                    Some([&AdditionalType::Track, &AdditionalType::Episode]),
                )
                .await?;

            match playback {
                Some(player) => {
                    let device = player.device;
                    self.target_device = Some(device);

                    Ok(())
                }
                None => Err(Error::msg("no playback device available")),
            }
        } else {
            Ok(())
        }
    }

    async fn advance_to_next_track(&mut self) -> Result<()> {
        if let Some(spotify) = self.spotify().await {
            let playstate_response = spotify
                .current_playback(
                    None,
                    Some([&AdditionalType::Track, &AdditionalType::Episode]),
                )
                .await?;

            if playstate_response.is_none() {
                return Err(anyhow::Error::msg("player state is unavailable"));
            }
            let playerstate = playstate_response.unwrap();

            let front = self.queue.pop_front();
            if let Some(track_info) = front {
                self.setup_next_track(track_info, self.device_id()).await;
            } else {
                println!("no track to play");
            }
        }

        Ok(())
    }

    fn device_id(&self) -> &str {
        self.target_device.as_ref().unwrap().id.as_ref().unwrap()
    }

    async fn start(&mut self) {
        self.find_target_device().await;
        if let Some(track) = self.queue.pop_front() {
            if let Some(spotify) = self.spotify().await {
                self.currently_playing = Some(InProgressTrack {
                    track: track.clone(),
                    start_instant: Instant::now(),
                });
                let device_id = self.device_id();
                self.setup_next_track(track, device_id).await;
                spotify.next_track(Some(device_id)).await.unwrap();
            }
        }
    }

    async fn setup_next_track(&self, track: TrackInfo, device_id: &str) {
        if let Some(spotify) = self.spotify().await {
            spotify
                .add_item_to_queue(&track.id, Some(device_id))
                .await
                .unwrap();

            let tx_clone = self.cmd_tx.clone();
            tokio::task::spawn(async move {
                println!(
                    "waking player thread in {} seconds",
                    (track.duration - Duration::from_secs(10)).as_secs()
                );
                tokio::time::sleep(track.duration - Duration::from_secs(10)).await;
                tx_clone.send(PlayerCommand::Wake).await.unwrap();
            });
        }
    }

    async fn add_track_to_queue(&mut self, track_id: TrackId) -> Result<()> {
        if let Some(spotify) = self.spotify().await {
            let full_track = spotify.track(&track_id).await?;
            self.queue.push_back(full_track.into());
        }
        Ok(())
    }

    async fn get_currently_playing(&self) -> Result<Option<TrackInfo>> {
        if let Some(spotify) = self.spotify().await {
            let playstate_response: Option<CurrentPlaybackContext> = spotify
                .current_playback(
                    None,
                    Some([&AdditionalType::Track, &AdditionalType::Episode]),
                )
                .await?;

            Ok(playstate_response
                .map(|playersate| {
                    playersate
                        .item
                        .map(|item| match item {
                            PlayableItem::Track(full_track) => Some(TrackInfo::from(full_track)),
                            PlayableItem::Episode(_) => None,
                        })
                        .flatten()
                })
                .flatten())
        } else {
            Ok(None)
        }
    }

    fn get_queued_tracks(&self) -> Vec<TrackInfo> {
        return self.queue.iter().map(|info| info.clone()).collect();
    }
}

pub fn start_player_thread(auth_state: ManagedAuthState) -> PlayerCommader {
    let (player, tx) = PlayerState::new(auth_state);
    tokio::task::spawn(player_task(player));
    PlayerCommader::new(tx)
}

async fn player_task(mut player: PlayerState) {
    println!("starting player task");

    loop {
        let cmd = player.cmd_rx.recv().await.unwrap(); //it's pretty bad if the channel has been droppped
        match cmd {
            PlayerCommand::Wake => {
                let result = player.advance_to_next_track().await;
                if let Err(err) = result {
                    println!("failed to advance track: {}", err);
                }
            }
            PlayerCommand::AddTrack(track_id) => {
                let result = player.add_track_to_queue(track_id).await;
                if let Err(err) = result {
                    println!("failed to add track to queue: {}", err);
                }
            }

            PlayerCommand::GetCurrentTrack(response_channel) => {
                let current_track = player.get_currently_playing().await;
                match current_track {
                    Ok(res) => {
                        response_channel.send(res).unwrap();
                    }
                    Err(err) => {
                        println!("failed to find current track: {}", err);
                    }
                }
            }
            PlayerCommand::GetTrackQueue(response_channel) => {
                let outvec = player.get_queued_tracks();
                response_channel.send(outvec).unwrap();
            }
            PlayerCommand::Start => {
                player.start().await;
            }
        }
    }
}

fn item_duration_almost_done(item: &Option<PlayableItem>, progress: &Option<Duration>) -> bool {
    if item.is_none() || progress.is_none() {
        //If there's nothing playing, we should probably just skip to the next song
        return true;
    }

    let total_duration = match item.as_ref().unwrap() {
        PlayableItem::Track(track) => track.duration,
        PlayableItem::Episode(episode) => episode.duration,
    };

    return total_duration - progress.unwrap() < Duration::from_secs(15);
}

struct InProgressTrack {
    track: TrackInfo,
    start_instant: Instant,
}

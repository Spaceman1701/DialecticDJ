use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Error, Result};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{AdditionalType, Device, PlayableItem, TrackId},
    AuthCodeSpotify,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use crate::persistence::TrackInfo;

pub type PlayerCommandQueue = Sender<PlayerCommand>;

pub struct PlayerCommader {
    sender: PlayerCommandQueue,
}

impl PlayerCommader {
    fn new(sender: PlayerCommandQueue) -> PlayerCommader {
        return PlayerCommader { sender: sender };
    }

    pub async fn request_currently_playing_track(&self) -> Result<Option<TrackInfo>> {
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
}

struct PlayerState {
    spotify: Arc<AuthCodeSpotify>,
    queue: VecDeque<TrackInfo>,
    currently_playing: Option<InProgressTrack>,
    cmd_rx: Receiver<PlayerCommand>,
    cmd_tx: PlayerCommandQueue,
    target_device: Option<Device>,
}

#[derive(Debug)]
pub enum PlayerCommand {
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
    fn new(spotify: Arc<AuthCodeSpotify>) -> (PlayerState, PlayerCommandQueue) {
        let (tx, rx) = tokio::sync::mpsc::channel(64); //TODO: consider unbounded channel here
        (
            PlayerState {
                spotify: spotify,
                queue: VecDeque::new(),
                currently_playing: None,
                cmd_rx: rx,
                cmd_tx: tx.clone(),
                target_device: None,
            },
            tx,
        )
    }

    async fn find_target_device(&mut self) -> Result<()> {
        let playback = self
            .spotify
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
    }

    async fn advance_to_next_track(&mut self) -> Result<()> {
        let playstate_response = self
            .spotify
            .current_playback(
                None,
                Some([&AdditionalType::Track, &AdditionalType::Episode]),
            )
            .await?;

        if playstate_response.is_none() {
            return Err(anyhow::Error::msg("player state is unavailable"));
        }
        let playerstate = playstate_response.unwrap();

        if item_duration_almost_done(&playerstate.item, &playerstate.progress) {
            let front = self.queue.pop_front();
            if let Some(track_info) = front {
                self.setup_next_track(track_info, &playerstate.device.id.unwrap())
                    .await;
            } else {
                println!("no track to play");
            }
        }
        Ok(())
    }

    async fn setup_next_track(&self, track: TrackInfo, device_id: &str) {
        self.spotify
            .add_item_to_queue(&track.id, Some(device_id))
            .await
            .unwrap();

        let tx_clone = self.cmd_tx.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(track.duration - Duration::from_secs(10)).await;
            tx_clone.send(PlayerCommand::Wake).await.unwrap();
        });
    }

    async fn add_track_to_queue(&mut self, track_id: TrackId) -> Result<()> {
        let full_track = self.spotify.track(&track_id).await?;
        self.queue.push_back(full_track.into());

        Ok(())
    }

    fn get_queued_tracks(&self) -> Vec<TrackInfo> {
        return self.queue.iter().map(|info| info.clone()).collect();
    }
}

pub fn start_player_thread(spotify: Arc<AuthCodeSpotify>) -> PlayerCommader {
    let (player, tx) = PlayerState::new(spotify);
    tokio::task::spawn(player_task(player));
    PlayerCommader::new(tx)
}

async fn player_task(mut player: PlayerState) {
    if let Err(err) = player.find_target_device().await {
        println!("ERROR: failed to start player task: {}", err);
        return;
    } else {
        println!(
            "connected to player device: {}",
            player.target_device.as_ref().unwrap().id.as_ref().unwrap()
        );
    }
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
                todo!()
            }
            PlayerCommand::GetTrackQueue(response_channel) => {
                let outvec = player.get_queued_tracks();
                response_channel.send(outvec).unwrap();
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

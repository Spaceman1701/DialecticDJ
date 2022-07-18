use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{AdditionalType, PlayableItem, TrackId},
    AuthCodeSpotify,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::persistence::TrackInfo;

pub type PlayerCommandQueue = Sender<PlayerCommand>;

pub struct PlayerCommader {
    sender: Sender<PlayerCommand>,
}

impl PlayerCommader {
    pub async fn request_currently_playing_track(&self) -> Result<Option<TrackInfo>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.sender.send(PlayerCommand::GetCurrentTrack(tx)).await?;

        return Ok(rx.recv().await.flatten());
    }

    pub async fn add_track_to_queue(&self, track_id: TrackId) -> Result<()> {
        self.sender.send(PlayerCommand::AddTrack(track_id)).await?;
        Ok(())
    }
}

struct PlayerState {
    spotify: Arc<AuthCodeSpotify>,
    queue: VecDeque<TrackInfo>,
    currently_playing: Option<InProgressTrack>,
    cmd_rx: Receiver<PlayerCommand>,
    cmd_tx: PlayerCommandQueue,
}

#[derive(Debug)]
pub enum PlayerCommand {
    ///Wake and configure the next track. Sent on a timer
    Wake,

    ///Add a track suggestion to the queue
    AddTrack(TrackId),

    //Return the currently playing track using the sender
    GetCurrentTrack(Sender<Option<TrackInfo>>),
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
            },
            tx,
        )
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
}

pub fn start_player_thread(spotify: Arc<AuthCodeSpotify>) -> PlayerCommandQueue {
    let (player, tx) = PlayerState::new(spotify);
    tokio::task::spawn(player_task(player));
    tx
}

async fn player_task(mut player: PlayerState) {
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

            PlayerCommand::GetCurrentTrack(response_channel) => {}
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

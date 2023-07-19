use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;

use poise::async_trait;
use poise::serenity_prelude::{ChannelId, CreateEmbed, Error, Http};
use songbird::input::Metadata;
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, EventContext, EventHandler as VoiceEventHandler};
use tokio::sync::Mutex;

pub fn check_msg<T>(result: Result<T, Error>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub fn bold(s: String) -> String {
    format!("**{}**", s)
}

pub fn hyperlink(text: String, url: String) -> String {
    format!("[{}]({})", text, url)
}

pub fn format_duration(d: Duration) -> String {
    let minutes = d.as_secs() / 60;
    let seconds = d.as_secs() % 60;
    format!("{:0>2}:{:0>2}", minutes, seconds)
}

pub trait SongEmbedBuilder {
    fn build_embed_queued_up(
        &mut self,
        metadata: Metadata,
        position_in_queue: u64,
        seconds_until: u64,
    ) -> &mut Self;

    fn build_embed_currently_playing(
        &mut self,
        metadata: Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self;

    fn build_embed_empty_queue(&mut self) -> &mut Self;

    fn build_current_queue_embed(&mut self, tracks: Vec<TrackHandle>) -> &mut Self;
}

impl SongEmbedBuilder for CreateEmbed {
    fn build_embed_queued_up(
        &mut self,
        metadata: Metadata,
        position_in_queue: u64,
        seconds_until: u64,
    ) -> &mut Self {
        self.color(0xA877C8)
            .title("New song queued up")
            .description(bold(hyperlink(
                metadata.title.unwrap(),
                metadata.source_url.unwrap(),
            )))
            .fields(vec![
                (
                    "Duration",
                    format_duration(metadata.duration.unwrap()),
                    false,
                ),
                (
                    "Position in queue",
                    if position_in_queue == 1 {
                        "Next".to_string()
                    } else {
                        position_in_queue.to_string()
                    },
                    false,
                ),
                (
                    "Estimated time until song is played",
                    format_duration(Duration::from_secs(seconds_until)),
                    false,
                ),
            ])
            .thumbnail(metadata.thumbnail.unwrap_or(
                "https://images.pexels.com/photos/11733110/pexels-photo-11733110.jpeg".to_string(),
            ))
    }

    fn build_embed_currently_playing(
        &mut self,
        metadata: Metadata,
        seconds_elapsed: Duration,
    ) -> &mut Self {
        self.color(0xA877C8)
            .description(format!(
                "Now playing: {}",
                bold(hyperlink(
                    metadata.title.unwrap(),
                    metadata.source_url.unwrap(),
                ))
            ))
            .fields(vec![(
                "Time remaining",
                format_duration(metadata.duration.unwrap().sub(seconds_elapsed)),
                false,
            )])
            .thumbnail(metadata.thumbnail.unwrap_or(
                "https://images.pexels.com/photos/11733110/pexels-photo-11733110.jpeg".to_string(),
            ))
    }

    fn build_embed_empty_queue(&mut self) -> &mut Self {
        self.color(0xED4245)
            .description("There are no songs in the queue")
    }

    fn build_current_queue_embed(&mut self, tracks: Vec<TrackHandle>) -> &mut Self {
        self.color(0xA877C8).title("Songs queued up").description(
            tracks
                .iter()
                .enumerate()
                .map(|(i, track)| {
                    let metadata = track.metadata().clone();
                    format!(
                        "{}. {}\n",
                        i + 1,
                        bold(hyperlink(
                            metadata.title.unwrap(),
                            metadata.source_url.unwrap()
                        ))
                    )
                })
                .collect::<String>(),
        )
    }
}

pub struct TrackEndNotifier {
    pub channel_id: ChannelId,
    pub http: Arc<Http>,
    pub handle_lock: Arc<Mutex<Call>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_tracks) = ctx {
            let handler = self.handle_lock.lock().await;
            if let Some(np) = handler.queue().current() {
                let elapsed = np.get_info().await.unwrap().play_time;
                check_msg(
                    self.channel_id
                        .send_message(&self.http, |m| {
                            m.embed(|e| {
                                e.build_embed_currently_playing(np.metadata().clone(), elapsed)
                            })
                        })
                        .await,
                );
            } else {
                check_msg(
                    self.channel_id
                        .send_message(&self.http, |m| m.embed(|e| e.build_embed_empty_queue()))
                        .await,
                );
            }
        }
        None
    }
}

use kdapp::episode::{Episode, EpisodeError, PayloadMetadata};

pub trait TimeBoundedEpisode: Episode {
    fn is_expired(&self, metadata: &PayloadMetadata) -> bool;
    fn time_remaining(&self, metadata: &PayloadMetadata) -> u64;
    fn auto_finalize(&mut self) -> Result<(), EpisodeError<Self::CommandError>>;
}

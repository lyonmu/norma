use std::sync::mpsc::{self, Receiver, Sender};

use crate::config::NormaConfig;
use crate::skills::SkillIndex;

#[derive(Debug, Clone)]
pub enum RuntimeUpdate {
    ConfigApplied(NormaConfig),
    ConfigRejected(String),
    SkillsApplied(SkillIndex),
    SkillsRejected(String),
}

pub fn runtime_update_channel() -> (Sender<RuntimeUpdate>, Receiver<RuntimeUpdate>) {
    mpsc::channel()
}

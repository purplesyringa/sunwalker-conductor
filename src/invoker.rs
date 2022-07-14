use crate::message;
use anyhow::{bail, Result};

pub struct Invoker {
    name: String,
}

impl Invoker {
    pub fn new(handshake: message::i2c::Handshake) -> Invoker {
        Invoker {
            name: handshake.invoker_name,
        }
    }

    pub async fn handle_message(&self, message: message::i2c::Message) -> Result<()> {
        use message::i2c::Message::*;
        match message {
            Handshake(message) => {
                bail!("Unexpected handshake in the middle of conversation: {message:?}");
            }
            UpdateMode(message) => self.update_mode(message).await,
            NotifyCompilationStatus(message) => self.notify_compilation_status(message).await,
            NotifyTestStatus(message) => self.notify_test_status(message).await,
            NotifySubmissionError(message) => self.notify_submission_error(message).await,
            RequestFile(message) => self.request_file(message).await,
        }
    }

    async fn update_mode(&self, message: message::i2c::UpdateMode) -> Result<()> {
        Ok(())
    }

    async fn notify_compilation_status(
        &self,
        message: message::i2c::NotifyCompilationStatus,
    ) -> Result<()> {
        Ok(())
    }

    async fn notify_test_status(&self, message: message::i2c::NotifyTestStatus) -> Result<()> {
        Ok(())
    }

    async fn notify_submission_error(
        &self,
        message: message::i2c::NotifySubmissionError,
    ) -> Result<()> {
        Ok(())
    }

    async fn request_file(&self, message: message::i2c::RequestFile) -> Result<()> {
        Ok(())
    }
}

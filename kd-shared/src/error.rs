use thiserror::Error;
use crate::rtp::RtpError;

#[derive(Debug, Error, PartialEq)]
pub enum KdError
{
    #[error("RTP error")]
    RtpError(#[from] RtpError),
}

pub type Result<T> = std::result::Result<T, KdError>;
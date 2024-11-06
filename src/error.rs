use crate::ibc::channel::IBC_CUSTOM_PROTOCOL_VERSION;
use cosmwasm_std::{Coin, StdError};
use cw2::VersionError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("price cannot be zero")]
    ZeroPrice,
    #[error("missing payment {:?}", missing_payment)]
    MissingPayment { missing_payment: Coin },
    #[error("{0}")]
    Version(#[from] VersionError),
    #[error("Unsupported ibc version on channel: {version}. Only: {IBC_CUSTOM_PROTOCOL_VERSION}")]
    InvalidIbcVersion { version: String },
    #[error("Only supports unordered channels")]
    OrderedChannel,
    #[error("Channel {channel_id} already exists")]
    ChannelAlreadyExists { channel_id: String },
    #[error("The channel cant be closed")]
    ChannelClosingNotAllowed,
    #[error("Unkown channel {channel_id}")]
    UnkownChannel { channel_id: String },
    #[error("Only token owner can send on IBC")]
    OnlyOwnerCanIBCSend,
}

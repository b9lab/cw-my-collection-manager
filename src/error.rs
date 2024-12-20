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
}

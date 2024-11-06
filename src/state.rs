use cosmwasm_schema::cw_serde;
use cosmwasm_std::IbcEndpoint;
use cw_storage_plus::{Item, Map};

use crate::msg::PaymentParams;

pub const CONTRACT_NAME: &str = "my-collection-manager";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct ChannelInfo {
    pub channel_id: String,
    pub counterparty_endpoint: IbcEndpoint,
    pub connection_id: String,
}

pub const PAYMENT_PARAMS: Item<PaymentParams> = Item::new("payment_params");
pub const IBC_CHANNEL_INFOS: Map<&str, ChannelInfo> = Map::new("ibc_channel_infos");

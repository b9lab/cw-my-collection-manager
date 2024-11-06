use crate::{
    error::ContractError,
    state::{ChannelInfo, IBC_CHANNEL_INFOS},
};
use cosmwasm_std::{
    entry_point, DepsMut, Env, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannel,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcOrder,
    Storage,
};

pub const IBC_CUSTOM_PROTOCOL_VERSION: &str = "ibc-name-transfer-1.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    check_channel_order_and_version(msg.channel(), msg.counterparty_version())?;
    check_channel_unknown(deps.storage, msg.channel())?;
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_CUSTOM_PROTOCOL_VERSION.to_owned(),
    }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    check_channel_order_and_version(msg.channel(), msg.counterparty_version())?;
    check_channel_unknown(deps.storage, msg.channel())?;
    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        channel_id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    IBC_CHANNEL_INFOS.save(deps.storage, &info.channel_id, &info)?;
    Ok(IbcBasicResponse::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    Err(ContractError::ChannelClosingNotAllowed)
}

fn check_channel_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version != IBC_CUSTOM_PROTOCOL_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(counterparty_version) = counterparty_version {
        if counterparty_version != IBC_CUSTOM_PROTOCOL_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: counterparty_version.to_string(),
            });
        }
    }
    if channel.order != IbcOrder::Unordered {
        return Err(ContractError::OrderedChannel);
    }
    Ok(())
}

fn check_channel_unknown(storage: &dyn Storage, channel: &IbcChannel) -> Result<(), ContractError> {
    if IBC_CHANNEL_INFOS.has(storage, &channel.endpoint.channel_id) {
        Err(ContractError::ChannelAlreadyExists {
            channel_id: channel.endpoint.channel_id.to_string(),
        })
    } else {
        Ok(())
    }
}

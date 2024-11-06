use crate::{
    error::ContractError,
    msg::{CollectionExecuteMsg, IbcMessage},
    state::{IBC_CHANNEL_INFOS, VOUCHERS_COLLECTION_ADDR},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, DepsMut, Env, Event, IbcBasicResponse, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, StdAck, Storage, WasmMsg,
};

pub fn voucher_token_id(channel_id: &String, collection: &String, token_id: &String) -> String {
    // hash this token path? which algorithm?
    format!("transfer_name/ibc/{channel_id}/{collection}/{token_id}")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    check_channel_exists(deps.storage, &msg.packet.src.channel_id)?;
    let ibc_msg = from_json::<IbcMessage>(msg.packet.data)?;
    let response = match ibc_msg {
        IbcMessage::TransferName {
            collection,
            token_id,
            sender_addr: _sender_addr,
            receiver_addr,
        } => ibc_receive_transfer_name(
            deps,
            env,
            msg.packet.src.channel_id,
            collection,
            token_id,
            receiver_addr,
        ),
        IbcMessage::ReturnName {
            collection,
            token_id,
            sender_addr: _sender_addr,
            receiver_addr,
        } => ibc_receive_return_name(deps, env, collection, token_id, receiver_addr),
    };
    match response {
        Ok(response) => Ok(response),
        Err(error) => Ok(IbcReceiveResponse::new()
            .add_attribute("method", "ibc_packet_receive")
            .add_attribute("error", error.to_string())
            .set_ack(StdAck::Error(error.to_string()))),
    }
}

fn ibc_receive_transfer_name(
    deps: DepsMut,
    _env: Env,
    channel_id: String,
    collection: String,
    token_id: String,
    receiver_addr: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let voucher_collection = VOUCHERS_COLLECTION_ADDR.load(deps.storage)?;
    let voucher_token_id = voucher_token_id(&channel_id, &collection, &token_id);
    let mint_msg = CollectionExecuteMsg::Mint {
        token_id: voucher_token_id,
        owner: receiver_addr,
        token_uri: None,
        extension: None,
    };
    let exec_msg = WasmMsg::Execute {
        contract_addr: voucher_collection,
        msg: to_json_binary(&mint_msg)?,
        funds: vec![],
    };
    let mint_event = Event::new("ibc-voucher-mint")
        .add_attribute("channel", channel_id)
        .add_attribute("original-collection", collection)
        .add_attribute("token_id", token_id);
    Ok(IbcReceiveResponse::new()
        .add_message(exec_msg)
        .add_event(mint_event))
}

fn ibc_receive_return_name(
    _deps: DepsMut,
    _env: Env,
    collection: String,
    token_id: String,
    receiver_addr: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let unescrow_msg = CollectionExecuteMsg::TransferNft {
        token_id,
        recipient: receiver_addr,
    };
    let exec_msg = WasmMsg::Execute {
        contract_addr: collection,
        msg: to_json_binary(&unescrow_msg)?,
        funds: vec![],
    };
    Ok(IbcReceiveResponse::new().add_message(exec_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    ack: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    check_channel_exists(deps.storage, &ack.original_packet.src.channel_id)?;
    let ack_data = from_json::<StdAck>(&ack.acknowledgement.data)
        .unwrap_or_else(|_| StdAck::Error(ack.acknowledgement.data.to_base64()));
    let original_msg = from_json::<IbcMessage>(ack.original_packet.data)?;
    match original_msg {
        IbcMessage::TransferName {
            collection,
            token_id,
            sender_addr,
            receiver_addr: _receiver_addr,
        } => match ack_data {
            StdAck::Error(_) => {
                ibc_unescrow_failed_transfer(deps, env, collection, token_id, sender_addr)
            }
            StdAck::Success(_) => Ok(IbcBasicResponse::default()),
        },
        IbcMessage::ReturnName {
            collection,
            token_id,
            sender_addr,
            receiver_addr: _receiver_addr,
        } => match ack_data {
            StdAck::Error(_) => ibc_unescrow_failed_return(
                deps,
                env,
                ack.original_packet.src.channel_id,
                collection,
                token_id,
                sender_addr,
            ),
            StdAck::Success(_) => ibc_burn_successful_return(
                deps,
                env,
                ack.original_packet.src.channel_id,
                collection,
                token_id,
            ),
        },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    check_channel_exists(deps.storage, &msg.packet.src.channel_id)?;
    match from_json::<IbcMessage>(msg.packet.data)? {
        IbcMessage::TransferName {
            collection,
            token_id,
            sender_addr,
            receiver_addr: _receiver_addr,
        } => ibc_unescrow_failed_transfer(deps, env, collection, token_id, sender_addr),
        IbcMessage::ReturnName {
            collection,
            token_id,
            sender_addr,
            receiver_addr: _receiver_addr,
        } => ibc_unescrow_failed_return(
            deps,
            env,
            msg.packet.src.channel_id,
            collection,
            token_id,
            sender_addr,
        ),
    }
}

fn check_channel_exists(storage: &dyn Storage, channel_id: &String) -> Result<(), ContractError> {
    if IBC_CHANNEL_INFOS.has(storage, channel_id) {
        Ok(())
    } else {
        Err(ContractError::UnkownChannel {
            channel_id: channel_id.to_string(),
        })
    }
}

fn ibc_unescrow_failed_transfer(
    _deps: DepsMut,
    _env: Env,
    collection: String,
    token_id: String,
    sender_addr: String,
) -> Result<IbcBasicResponse, ContractError> {
    let unescrow_msg = CollectionExecuteMsg::TransferNft {
        token_id,
        recipient: sender_addr,
    };
    let exec_msg = WasmMsg::Execute {
        contract_addr: collection,
        msg: to_json_binary(&unescrow_msg)?,
        funds: vec![],
    };
    Ok(IbcBasicResponse::new().add_message(exec_msg))
}

fn ibc_burn_successful_return(
    deps: DepsMut,
    _env: Env,
    channel_id: String,
    collection: String,
    token_id: String,
) -> Result<IbcBasicResponse, ContractError> {
    let voucher_collection: String = VOUCHERS_COLLECTION_ADDR.load(deps.storage)?;
    let voucher_token_id = voucher_token_id(&channel_id, &collection, &token_id);
    let burn_msg = CollectionExecuteMsg::Burn {
        token_id: voucher_token_id,
    };
    let exec_msg = WasmMsg::Execute {
        contract_addr: voucher_collection,
        msg: to_json_binary(&burn_msg)?,
        funds: vec![],
    };
    let escrow_event = Event::new("ibc-voucher-burn")
        .add_attribute("channel", channel_id.to_owned())
        .add_attribute("original-collection", collection.to_owned())
        .add_attribute("token_id", token_id.to_owned());
    Ok(IbcBasicResponse::default()
        .add_message(exec_msg)
        .add_event(escrow_event))
}

fn ibc_unescrow_failed_return(
    deps: DepsMut,
    _env: Env,
    channel_id: String,
    collection: String,
    token_id: String,
    sender_addr: String,
) -> Result<IbcBasicResponse, ContractError> {
    let voucher_collection = VOUCHERS_COLLECTION_ADDR.load(deps.storage)?;
    let voucher_token_id = voucher_token_id(&channel_id, &collection, &token_id);
    let unescrow_msg = CollectionExecuteMsg::TransferNft {
        token_id: voucher_token_id,
        recipient: sender_addr,
    };
    let exec_msg = WasmMsg::Execute {
        contract_addr: voucher_collection,
        msg: to_json_binary(&unescrow_msg)?,
        funds: vec![],
    };
    Ok(IbcBasicResponse::new().add_message(exec_msg))
}

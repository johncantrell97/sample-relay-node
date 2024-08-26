use axum::{
    extract::{Path, State},
    Json,
};
use hex::prelude::*;
use hex::DisplayHex;
use ldk_node::{
    bitcoin::secp256k1::PublicKey,
    lightning::ln::{msgs::SocketAddress, PaymentHash},
    lightning_invoice::Bolt11Invoice,
    ChannelDetails,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingAddress {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenChannelRequest {
    pub pubkey: PublicKey,
    pub ip_port: String,
    pub funding_sats: u64,
    pub push_sats: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenChannelResponse {
    pub user_channel_id: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectPeerRequest {
    pub pubkey: PublicKey,
    pub ip_port: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectPeerResponse {}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
  node_id: String,
  address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeersResponse {
  peers: Vec<Peer>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactChannel {
    pub channel_id: String,
    pub counterparty_node_id: PublicKey,
    pub channel_value_sats: u64,
    pub user_channel_id: u128,
    pub outbound_capacity_msat: u64,
    pub inbound_capacity_msat: u64,
    pub is_channel_ready: bool,
    pub is_usable: bool,
}

impl From<ChannelDetails> for CompactChannel {
    fn from(channel: ChannelDetails) -> Self {
        Self {
            channel_id: channel.channel_id.to_string(),
            counterparty_node_id: channel.counterparty_node_id,
            channel_value_sats: channel.channel_value_sats,
            user_channel_id: channel.user_channel_id.0,
            outbound_capacity_msat: channel.outbound_capacity_msat,
            inbound_capacity_msat: channel.inbound_capacity_msat,
            is_channel_ready: channel.is_channel_ready,
            is_usable: channel.is_usable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListChannelsResponse {
    pub channels: Vec<CompactChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayInvoiceRequest {
    pub invoice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayInvoiceResponse {
    pub payment_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInvoiceRequest {
    pub amount_sats: u64,
    pub description: String,
    pub expiry_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInvoiceResponse {
    pub invoice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBalanceResponse {
    pub total_onchain_balance_sats: u64,
    pub spendable_onchain_balance_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPaymentResponse {
    pub status: String,
    pub preimage: Option<String>,
}

pub(crate) async fn funding_address(State(state): State<AppState>) -> Json<FundingAddress> {
    Json(FundingAddress {
        address: state.node.new_onchain_address().unwrap().to_string(),
    })
}

pub(crate) async fn open_channel(
    State(state): State<AppState>,
    Json(req): Json<OpenChannelRequest>,
) -> Json<OpenChannelResponse> {
    let socket_addr = SocketAddress::from_str(&req.ip_port).unwrap();
    let res = state
        .node
        .connect_open_channel(
            req.pubkey,
            socket_addr,
            req.funding_sats,
            Some(req.push_sats * 1000),
            None,
            true,
        )
        .unwrap();

    Json(OpenChannelResponse {
        user_channel_id: res.0,
    })
}

pub(crate) async fn connect_peer(
  State(state): State<AppState>,
  Json(req): Json<ConnectPeerRequest>,
) -> Json<ConnectPeerResponse> {
  let socket_addr = SocketAddress::from_str(&req.ip_port).unwrap();
  let _ = state
      .node
      .connect(
          req.pubkey,
          socket_addr,
          false,
      )
      .unwrap();

  Json(ConnectPeerResponse {})
}

pub(crate) async fn list_peers(
  State(state): State<AppState>,
) -> Json<ListPeersResponse> {
  let peers = state.node.list_peers();
  Json(ListPeersResponse {
    peers: peers.into_iter().map(|peer| {
      Peer {
        address: peer.address.to_string(),
        node_id: peer.node_id.to_string(),
      }
    }).collect()
  })
}

pub(crate) async fn list_channels(State(state): State<AppState>) -> Json<ListChannelsResponse> {
    let channels = state
        .node
        .list_channels()
        .into_iter()
        .map(|channel| channel.into())
        .collect::<Vec<_>>();

    Json(ListChannelsResponse { channels })
}

pub(crate) async fn pay_invoice(
    State(state): State<AppState>,
    Json(req): Json<PayInvoiceRequest>,
) -> Json<PayInvoiceResponse> {
    let invoice = Bolt11Invoice::from_str(&req.invoice).unwrap();
    let res = state.node.send_payment(&invoice).unwrap();
    Json(PayInvoiceResponse {
        payment_hash: res.0.to_lower_hex_string(),
    })
}

pub(crate) async fn get_invoice(
    State(state): State<AppState>,
    Json(req): Json<GetInvoiceRequest>,
) -> Json<GetInvoiceResponse> {
    let invoice = state
        .node
        .receive_payment(req.amount_sats * 1000, &req.description, req.expiry_secs)
        .unwrap();

    Json(GetInvoiceResponse {
        invoice: invoice.to_string(),
    })
}

pub(crate) async fn sync(State(state): State<AppState>) -> Json<Value> {
    state.node.sync_wallets().unwrap();
    Json(json!({"synced": true}))
}

pub(crate) async fn get_balance(State(state): State<AppState>) -> Json<GetBalanceResponse> {
    let balances = state.node.list_balances();
    Json(GetBalanceResponse {
        total_onchain_balance_sats: balances.total_onchain_balance_sats,
        spendable_onchain_balance_sats: balances.spendable_onchain_balance_sats,
    })
}

pub(crate) async fn get_payment(
    State(state): State<AppState>,
    Path(payment_hash): Path<String>,
) -> Json<GetPaymentResponse> {
    let payment_hash_bytes = <[u8; 32]>::from_hex(&payment_hash).unwrap();
    let payment_hash = PaymentHash(payment_hash_bytes);
    let payment = state.node.payment(&payment_hash).unwrap();

    Json(GetPaymentResponse {
        status: match payment.status {
            ldk_node::PaymentStatus::Pending => "pending".to_string(),
            ldk_node::PaymentStatus::Succeeded => "succeeded".to_string(),
            ldk_node::PaymentStatus::Failed => "failed".to_string(),
        },
        preimage: payment
            .preimage
            .map(|preimage| preimage.0.to_lower_hex_string()),
    })
}

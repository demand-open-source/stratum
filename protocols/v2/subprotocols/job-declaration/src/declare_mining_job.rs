use alloc::vec::Vec;
use binary_sv2::{binary_codec_sv2, Deserialize, Seq064K, Serialize, Str0255, B0255, B064K, U256};
use core::convert::TryInto;

/// Message used by JDC to proposes a selected set of transactions to JDS they wish to
/// mine on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DeclareMiningJob<'decoder> {
    /// A unique identifier for this request.
    ///
    /// Used for pairing request/response.
    pub request_id: u32,
    /// Token received previously through [`crate::AllocateMiningJobTokenSuccess`] message.
    pub mining_job_token: B0255<'decoder>,
    /// Header version field.
    pub version: u32,
    /// The coinbase transaction nVersion field
    pub coinbase_prefix: B064K<'decoder>,
    /// Up to 8 bytes (not including the length byte) which are to be placed at the beginning of
    /// the coinbase field in the coinbase transaction.
    pub coinbase_suffix: B064K<'decoder>,
    // List of transactions for that job in the same order that will apppear in the block.
    pub tx_list: Seq064K<'decoder, U256<'decoder>>,
    /// Extra data which the JDS may require to validate the work.
    pub excess_data: B064K<'decoder>,
}

/// Messaged used by JDS to accept [`DeclareMiningJob`] message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DeclareMiningJobSuccess<'decoder> {
    /// A unique identifier for this request.
    ///
    /// Must be the same as the received [`DeclareMiningJob::request_id`].
    pub request_id: u32,
    /// This **may** be the same token as [DeclareMiningJob::mining_job_token] if the pool allows
    /// to start mining on a non declared job. If the token is different (irrespective of if the
    /// downstream is already mining using it), the downstream **must** send a `SetCustomMiningJob`
    /// message on each connection which wishes to mine using the declared job.
    pub new_mining_job_token: B0255<'decoder>,
}

/// Messaged used by JDS to reject [`DeclareMiningJob`] message.
///
/// Downstream should consider this as a trigger to fallback into some other Pool/JDS or solo
/// mining.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DeclareMiningJobError<'decoder> {
    /// The unique identifier of the request.
    ///
    /// Must be the same as the received [`DeclareMiningJob::request_id`].
    pub request_id: u32,
    /// Possible values:
    ///
    /// - invalid-mining-job-token
    /// - invalid-job-param-value-{DeclareMiningJob::field}
    pub error_code: Str0255<'decoder>,
    /// Optional details about the error.
    pub error_details: B064K<'decoder>,
}

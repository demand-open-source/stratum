use alloc::vec::Vec;
use binary_sv2::{binary_codec_sv2, Deserialize, Serialize, Str0255, B032};
use core::{convert::TryInto, fmt};

/// Message used by downstream to send result of its hashing work to an upstream.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitSharesStandard {
    /// Channel identification.
    pub channel_id: u32,
    /// Unique sequential identifier of the submit within the channel.
    pub sequence_number: u32,
    /// Identifier of the job as provided by [`NewMiningJob`] or [`NewExtendedMiningJob`] message.
    ///
    /// [`NewMiningJob`]: crate::NewMiningJob
    /// [`NewExtendedMiningJob`]: crate::NewExtendedMiningJob
    pub job_id: u32,
    /// Nonce leading to the hash being submitted.
    pub nonce: u32,
    /// The `nTime` field in the block header. This must be greater than or equal to the
    /// `header_timestamp` field in the latest [`SetNewPrevHash`] message and lower than or equal
    /// to that value plus the number of seconds since the receipt of that message.
    ///
    /// [`SetNewPrevHash`]: crate::SetNewPrevHash
    pub ntime: u32,
    /// Full `nVersion` field.
    pub version: u32,
}

/// Message used by downstream to send result of its hashing work to an upstream.
///
/// The message is the same as [`SubmitShares`], but with an additional field,
/// [`SubmitSharesExtended::extranonce`].
///
/// Only relevant for Extended Channels.
#[derive(Serialize, Deserialize, Clone)]
pub struct SubmitSharesExtended<'decoder> {
    /// Channel identification.
    pub channel_id: u32,
    /// Unique sequential identifier of the submit within the channel.
    pub sequence_number: u32,
    /// Identifier of the job as provided by [`NewMiningJob`] or [`NewExtendedMiningJob`] message.
    ///
    /// [`NewMiningJob`]: crate::NewMiningJob
    /// [`NewExtendedMiningJob`]: crate::NewExtendedMiningJob
    pub job_id: u32,
    /// Nonce leading to the hash being submitted.
    pub nonce: u32,
    /// The nTime field in the block header. This must be greater than or equal to the
    /// `header_timestamp` field in the latest [`SetNewPrevHash`] message and lower than or equal
    /// to that value plus the number of seconds since the receipt of that message.
    ///
    /// [`SetNewPrevHash`]: crate::SetNewPrevHash
    pub ntime: u32,
    /// Full nVersion field.
    pub version: u32,
    /// Extranonce bytes which need to be added to the coinbase tx to form a fully valid submission
    /// (`full coinbase = coinbase_tx_prefix + extranonce_prefix + extranonce +
    /// coinbase_tx_suffix`).
    ///
    /// The size of the provided extranonce must be equal to the negotiated extranonce size from
    /// channel opening flow.
    pub extranonce: B032<'decoder>,
}

impl<'decoder> fmt::Debug for SubmitSharesExtended<'decoder> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmitSharesExtended")
            .field("channel_id", &self.channel_id)
            .field("sequence_number", &self.sequence_number)
            .field("job_id", &self.job_id)
            .field("nonce", &self.nonce)
            .field("ntime", &self.ntime)
            .field("version", &self.version)
            .field("extranonce_prefix", &self.extranonce.to_hex())
            .finish()
    }
}

/// Message used by upstream to accept [`SubmitSharesStandard`] or [`SubmitSharesExtended`].
///
/// Because it is a common case that shares submission is successful, this response can be provided
/// for multiple [`SubmitShare`] messages aggregated together.
///
/// The upstream doesn’t have to double check that the sequence numbers sent by a downstream are
/// actually increasing. It can use the last one received when sending a response. It is the
/// downstream’s responsibility to keep the sequence numbers correct/useful.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitSharesSuccess {
    /// Channel identifier.
    pub channel_id: u32,
    /// Most recent sequence number with a correct result.
    pub last_sequence_number: u32,
    /// Count of new submits acknowledged within this batch.
    pub new_submits_accepted_count: u32,
    /// Sum of shares acknowledged within this batch.
    pub new_shares_sum: u64,
}

/// Message used by upstream to reject [`SubmitSharesStandard`] or [`SubmitSharesExtended`].
///
/// In case the upstream is not able to immediately validate the submission, the error is sent as
/// soon as the result is known. This delayed validation can occur when a miner gets faster
/// updates about a new `prevhash` than the upstream does.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitSharesError<'decoder> {
    /// Channel identification.
    pub channel_id: u32,
    /// Unique sequential identifier of the submit within the channel.
    pub sequence_number: u32,
    /// Rejection reason.
    ///
    /// Possible error codes:
    ///
    /// - invalid-channel-id
    /// - stale-share
    /// - difficulty-too-low
    /// - invalid-job-id
    pub error_code: Str0255<'decoder>,
}

impl<'a> SubmitSharesError<'a> {
    pub fn invalid_channel_error_code() -> &'static str {
        "invalid-channel-id"
    }
    pub fn stale_share_error_code() -> &'static str {
        "stale-share"
    }
    pub fn difficulty_too_low_error_code() -> &'static str {
        "difficulty-too-low"
    }
    pub fn invalid_job_id_error_code() -> &'static str {
        "invalid-job-id"
    }
}

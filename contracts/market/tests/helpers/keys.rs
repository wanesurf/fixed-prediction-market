use clp_feed_interface::msg::PriceSubmission;
use ed25519_dalek::{Signature, SigningKey};
use signature::SignerMut;

#[derive(Clone)]
pub struct ValidatorKey {
    pub signing_key: SigningKey,
    pub public_key: String,
    pub id: String,
}

impl ValidatorKey {
    pub fn sign_price_submission(&mut self, submission: &PriceSubmission) -> Result<Signature, signature::Error> {
        let tx = format!(
            "{}:{}:{:.8}:{}:{}",
            submission.validator_id, submission.asset, submission.price, submission.timestamp, submission.sources.join(",")
        );
        Ok(self.signing_key.sign(tx.as_bytes()))
    }
}

pub fn signing_key_from_seed(seed: &str, id: &str) -> ValidatorKey {
    let key_bytes = hex::decode(seed)
        .expect("Invalid private key: must be a valid hex string");
        
    let signing_key = SigningKey::from_bytes(
        &key_bytes.as_slice().try_into()
            .expect("Invalid private key: failed to convert to signing key")
    );

    let public_key = hex::encode(signing_key.verifying_key().to_bytes());

    ValidatorKey {
        signing_key,
        public_key,
        id: id.to_string(),
    }
}

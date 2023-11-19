use duplicate::duplicate_item;
use prost::Message;
use ring::digest::{Context, SHA256};
use rsa::pkcs1v15::SigningKey;
use rsa::pss::{Signature, VerifyingKey};
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use crate::proto::block::{DataCapsuleFileSystemBlock, Id};

pub trait SignableBlock {
    fn sign(& mut self, key: &SigningKey<Sha256>);
    fn validate(& mut self, key: &VerifyingKey<Sha256>) -> bool;
}

#[duplicate_item(T; [Id]; [DataCapsuleFileSystemBlock])]
impl SignableBlock for T {
    fn sign(&mut self, key: &SigningKey<Sha256>) {
        self.signature = vec![];
        self.signature = sign_data(self, key);
    }

    fn validate(&mut self, key: &VerifyingKey<Sha256>) -> bool {
        let signature = self.signature.clone();
        self.signature = vec![];
        let result = validate_signature(self, key, &signature);
        self.signature = signature;
        return result;
    }
}

fn sign_data<T>(data: &T, key: &SigningKey<Sha256>) -> Vec<u8> where T: Message {
    let mut context = Context::new(&SHA256);
    let mut buf = vec![];
    data.encode(&mut buf).unwrap();
    context.update(&buf);
    return key.sign(&buf).to_vec();
}

fn validate_signature<T>(data: &T, key: &VerifyingKey<Sha256>, signature: &Vec<u8>) -> bool where T: Message {
    let mut context = Context::new(&SHA256);
    let mut buf = vec![];
    data.encode(&mut buf).unwrap();
    context.update(&buf);
    return key.verify(&buf, &Signature::try_from(signature.as_slice()).unwrap()).is_ok();
}
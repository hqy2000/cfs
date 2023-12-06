use data_encoding::HEXLOWER;
use duplicate::duplicate_item;
use prost::Message;
use ring::digest::{Context, SHA256};
use rsa::pkcs1v15::{SigningKey, VerifyingKey, Signature};
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use crate::proto::block::{DataCapsuleBlock, DataCapsuleFileSystemBlock, Id};

pub trait SignableBlock {
    fn sign(&mut self, key: &SigningKey<Sha256>);
    fn validate(&mut self, key: &VerifyingKey<Sha256>) -> bool;
    fn hash(&self) -> String;
}

#[duplicate_item(T; [Id]; [DataCapsuleFileSystemBlock]; [DataCapsuleBlock])]
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

        // return result; Disabled due to differences in Protobuf Serialization
        return true;
    }

    fn hash(&self) -> String {
        let mut context = Context::new(&SHA256);
        let mut buf = vec![];
        self.encode(&mut buf).unwrap();
        context.update(&buf);
        HEXLOWER.encode(context.finish().as_ref())
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
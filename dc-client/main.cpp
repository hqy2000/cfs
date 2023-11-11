#define FUSE_USE_VERSION 31

#include <iostream>
#include <string>
#include <sstream>
#include "message.pb.h"
#include "fuse3/fuse.h"
#include <openssl/evp.h>
#include <iomanip>
#include <optional>


//struct fuse_operations dcfs2_fuse_oper{
//
//};

std::string digest_message(std::string data) {
    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    EVP_DigestInit_ex(mdctx, EVP_sha256(), NULL);
    EVP_DigestUpdate(mdctx, data.data(), data.length());
    unsigned int digestLen = EVP_MD_size(EVP_sha256());
    auto digest = (unsigned char *) OPENSSL_malloc(digestLen);
    EVP_DigestFinal_ex(mdctx, digest, &digestLen);
    EVP_MD_CTX_free(mdctx);

    std::stringstream ss;
    for (int i = 0; i < digestLen; i++) {
        ss << std::hex << std::setw(2) << std::setfill('0') << (int) digest[i];
    }
    return ss.str();
}

int put(const Key& key, const Value& value) {
    return 0; // dummy return values for now.
}

std::optional<Value> get(const Key& key) {
    return std::nullopt; // dummy return values for now.
}

void put_acl() {
    Key key;
    ACLKey aclKey;
    aclKey.set_writeid("writer1");
    key.set_allocated_aclkey(&aclKey);

    Value value;
    ACLValue aclValue;
    ACL* acl = aclValue.add_acl();
    acl->set_publickey("8a8e1239773");
    acl->set_uid(1001);
    value.set_allocated_aclvalue(&aclValue);

    put(key, value);
}

void put_data() {
    Key key;
    DataKey dataKey;
    dataKey.set_path(0, "folder");
    dataKey.set_path(1, "example.txt");
    dataKey.set_isfolder(false);
    key.set_allocated_datakey(&dataKey);

    Value value;
    DataValue dataValue;
    dataValue.set_data("example txt data");
    Signature signature;
    signature.set_signature(digest_message(dataValue.data())); // todo: replace this with public key signing.
    signature.set_allocated_writer(nullptr); // todo: change to writer acl key
    signature.set_userid(1001);
    dataValue.set_allocated_signature(&signature);
    value.set_allocated_datavalue(&dataValue);

    put(key, value);
}

int main(int argc, char *argv[]) {
    std::cout << "dcfs2 middleware" << std::endl;


    put_acl();
    put_data();



    return 0;
}

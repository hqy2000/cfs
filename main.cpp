#define FUSE_USE_VERSION 31

#include <iostream>
#include <string>
#include <sstream>
#include "message.pb.h"
#include "fuse3/fuse.h"
#include <openssl/evp.h>
#include <iomanip>


struct fuse_operations dcfs2_fuse_oper{

};

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

int main(int argc, char *argv[]) {
    std::cout << "dcfs2" << std::endl;

    Message message;
    message.set_data("test");
    message.set_signature(digest_message(message.data()));
    std::cout << "data: " << message.data() << std::endl;
    std::cout << "hash: " << message.signature() << std::endl;

    return fuse_main(argc, argv, &dcfs2_fuse_oper, nullptr);
}

#define FUSE_USE_VERSION 31

#include <iostream>
#include <string>
#include <sstream>
#include "message.pb.h"
#include "fuse3/fuse.h"
#include <openssl/sha.h>
#include <iomanip>


struct fuse_operations dcfs2_fuse_oper {

};

std::string sha256(const std::string& str)
{
    unsigned char hash[SHA256_DIGEST_LENGTH];
    SHA256_CTX sha256;
    SHA256_Init(&sha256);
    SHA256_Update(&sha256, str.c_str(), str.size());
    SHA256_Final(hash, &sha256);
    std::stringstream ss;
    for(int i = 0; i < SHA256_DIGEST_LENGTH; i++)
    {
        ss << std::hex << std::setw(2) << std::setfill('0') << (int)hash[i];
    }
    return ss.str();
}

int main(int argc, char *argv[]) {
    std::cout<<"dcfs2"<<std::endl;

    Message message;
    message.set_data("test");
    message.set_signature(sha256(message.data()));
    std::cout<<"data: "<<message.data()<<std::endl;
    std::cout<<"hash: "<<message.signature()<<std::endl;

    return fuse_main(argc, argv, &dcfs2_fuse_oper, nullptr);
}

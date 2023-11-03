#define FUSE_USE_VERSION 31

#include <iostream>
#include "message.pb.h"
#include "fuse3/fuse.h"

struct fuse_operations dcfs2_fuse_oper {

};

int main(int argc, char *argv[]) {
    std::cout<<"dcfs2"<<std::endl;

    Message message;
    message.set_id(2);

    return fuse_main(argc, argv, &dcfs2_fuse_oper, nullptr);
}

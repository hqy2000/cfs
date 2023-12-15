# CapsuleFS

This repository contains the source code for CapsuleFS, a file system based on [DataCapsule](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2020/EECS-2020-10.pdf).

This README only contains instructions on how to deploy this file system. Please refer to our [page (Project #1)](https://people.eecs.berkeley.edu/~kubitron/courses/cs262a-F23/index_projects.html) to learn more about our design.

## Environment
To compile and run this program, you'll need to install the following dependencies. Most of them should be available through your operating system's package manager.

- Rust
- Go
- Protocol Buffer Compiler
- FUSE3
- OpenSSL

## How to Run
Follow these instructions to run this program.

### TLS
A TLS certificate and private key pair are required for communication between our client, server, and middleware. You can obtain one from [Let's Encrypt](https://letsencrypt.org/) or self-sign a pair.

### Keys
To generate the required keys for servers and clients, use the following commands:
```bash
openssl genrsa -out private_key.pem 2048 
openssl rsa -in private_key.pem -outform PEM -pubout -out public_key.pem
```
Each server or client will need one pair of RSA keys.

### DataCapsule Server
Run `src/bin/gen.rs` with `cargo` to generate the initial state of the DataCapsule. You'll need to specify the default ACL key on the command line. Remember to note the initial root hash, as you'll need it to update the configuration file.

A sample configuration file is provided at `config/server.json`. Be sure to update the configuration file, especially the keys and the data file.

Then, run `src/bin/server.rs` with `cargo` to start the server.

*Note: All changes to the data files are stored in memory.*

### Middleware

To compile the middleware, execute the following command in the `src/` directory:
```bash
protoc -I ../proto --go_out=. --go-grpc_out=. ../proto/*.proto --experimental_allow_proto3_optional
```

A sample configuration file is provided at `middleware.json`. Update the configuration file as necessary, particularly the keys and the data file.

Afterward, run `src/bin/middleware.go` with `go` to start the middleware.

### Client
A sample configuration file is available at `config/client.json`. Make sure to update the configuration file, especially the keys and the root hash.

Finally, run `src/bin/client.rs` with `cargo` to start the client. You can specify the mount point using the first argument.


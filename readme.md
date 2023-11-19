## Useful Commands

### Gen Keys
openssl genrsa -out client2_private.pem 2048
openssl rsa -in client2_private.pem -outform PEM -pubout -out client2_public.pem

### Gen ProtoBuf for go
Under **`src/`**: `protoc -I ../proto --go_out=. --go-grpc_out=. ../proto/*.proto --experimental_allow_proto3_optional`

package main

import (
	"crypto/rsa"
	"crypto/x509"
	"dcfs2/middleware/src/lib"
	"dcfs2/middleware/src/lib/go_proto"
	"encoding/pem"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"log"
	"net"
	"os"
)

func main() {
	lis, err := net.Listen("tcp", ":50060")
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	s := grpc.NewServer()
	go_proto.RegisterMiddlewareServer(s, &lib.MiddlewareServer{
		InodeClient: connect(":50052"),
		DataClient:  connect(":50051"),
		PrivateKey:  loadPrivateKey("key/server_private.pem"),
	})
	log.Printf("server listening at %v", lis.Addr())
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}

func loadPrivateKey(file string) *rsa.PrivateKey {
	data, err := os.ReadFile(file)
	if err != nil {
		panic(err)
	}
	block, _ := pem.Decode(data)
	parseResult, err := x509.ParsePKCS8PrivateKey(block.Bytes)
	if err != nil {
		panic(err)
	}
	key := parseResult.(*rsa.PrivateKey)
	return key
}

func connect(addr string) go_proto.DataCapsuleClient {
	conn, err := grpc.Dial(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("did not connect: %v", err)
	}
	return go_proto.NewDataCapsuleClient(conn)
}

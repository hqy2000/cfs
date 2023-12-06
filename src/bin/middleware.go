package main

import (
	"crypto/rsa"
	"crypto/x509"
	"dcfs2/middleware/src/lib"
	"dcfs2/middleware/src/lib/go_proto"
	"encoding/pem"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"log"
	"net"
	"os"
)

func main() {
	lis, err := net.Listen("tcp", "loopback.hqy.moe:50060")
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}

	creds, err := credentials.NewServerTLSFromFile("config/loopback.hqy.moe_fullchain.pem", "config/loopback.hqy.moe_privkey.pem")
	if err != nil {
		log.Fatalf("failed to load tls cert: %v", err)
	}

	s := grpc.NewServer(grpc.Creds(creds))

	go_proto.RegisterMiddlewareServer(s, &lib.MiddlewareServer{
		InodeClient: connect("loopback.hqy.moe:50052"),
		DataClient:  connect("loopback.hqy.moe:50051"),
		PrivateKey:  loadPrivateKey("config/server_private.pem"),
	})
	log.Println(s.GetServiceInfo())
	s.GetServiceInfo()
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
	creds, err := credentials.NewClientTLSFromFile("config/loopback.hqy.moe_fullchain.pem", "")
	if err != nil {
		log.Fatalf("failed to load tls cert: %v", err)
	}

	conn, err := grpc.Dial(addr, grpc.WithTransportCredentials(creds))
	if err != nil {
		log.Fatalf("did not connect: %v", err)
	}
	return go_proto.NewDataCapsuleClient(conn)
}

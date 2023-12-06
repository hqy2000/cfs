package main

import (
	"cfs/middleware/src/lib"
	"cfs/middleware/src/lib/go_proto"
	"crypto/rsa"
	"crypto/x509"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"log"
	"net"
	"os"
)

func main() {
	args := os.Args
	if len(args) != 2 {
		log.Fatalf("Usage: middleware [config file].")
	}

	var config Config
	raw, err := os.ReadFile(args[1])
	if err != nil {
		panic(err)
	}
	err = json.Unmarshal(raw, &config)
	if err != nil {
		panic(err)
	}

	lis, err := net.Listen("tcp", fmt.Sprintf("%s:%d", config.Address, config.Port))
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}

	creds, err := credentials.NewServerTLSFromFile(config.TLS.Certificate, config.TLS.PrivateKey)
	if err != nil {
		log.Fatalf("failed to load tls cert: %v", err)
	}

	s := grpc.NewServer(grpc.Creds(creds))

	go_proto.RegisterMiddlewareServer(s, &lib.MiddlewareServer{
		InodeClient:     connect(config.InodeServer.Url, config.InodeServer.TLS.CA),
		DataClient:      connect(config.DataServer.Url, config.InodeServer.TLS.CA),
		InodeSigningKey: loadPrivateKey(config.InodeServer.SigningKey),
		DataSigningKey:  loadPrivateKey(config.DataServer.SigningKey),
		EnableCrypto:    config.IsCryptoEnabled,
	})
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

func connect(addr string, ca string) go_proto.DataCapsuleClient {
	creds, err := credentials.NewClientTLSFromFile(ca, "")
	if err != nil {
		log.Fatalf("failed to load tls cert: %v", err)
	}

	conn, err := grpc.Dial(addr, grpc.WithTransportCredentials(creds))
	if err != nil {
		log.Fatalf("did not connect: %v", err)
	}
	return go_proto.NewDataCapsuleClient(conn)
}

type Config struct {
	IsCryptoEnabled bool `json:"isCryptoEnabled"`
	DataServer      struct {
		Url        string `json:"url"`
		SigningKey string `json:"signingKey"`
		TLS        struct {
			CA string `json:"ca"`
		} `json:"tls"`
	} `json:"dataServer"`
	InodeServer struct {
		Url        string `json:"url"`
		SigningKey string `json:"signingKey"`
		TLS        struct {
			CA string `json:"ca"`
		} `json:"tls"`
	} `json:"inodeServer"`
	Address string `json:"address"`
	Port    int    `json:"port"`
	TLS     struct {
		PrivateKey  string `json:"privateKey"`
		Certificate string `json:"certificate"`
	} `json:"tls"`
}

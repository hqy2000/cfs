package main

import (
	"context"
	pb "dcfs2/middleware/proto"
	"flag"
	"fmt"
	"log"
	"net"

	"google.golang.org/grpc"
)

var (
	port = flag.Int("port", 50051, "The server port")
)

type server struct {
	pb.UnimplementedMiddlewareServer
}

func (s *server) Put(ctx context.Context, in *pb.PutMiddlewareRequest) (*pb.PutMiddlewareResponse, error) {
	log.Printf("Received: %v", string(in.GetData()))
	log.Printf("Received: %v", string(in.GetSignature()))
	replyHash := "Hash response from dummy server"
	return &pb.PutMiddlewareResponse{Hash: replyHash}, nil
}

func main() {
	flag.Parse()
	lis, err := net.Listen("tcp", fmt.Sprintf(":%d", *port))
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	s := grpc.NewServer()
	pb.RegisterMiddlewareServer(s, &server{})
	log.Printf("server listening at %v", lis.Addr())
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}

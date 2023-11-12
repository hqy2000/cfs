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
	pb.UnimplementedWriteServerServer
}

func (s *server) Write(ctx context.Context, in *pb.WriteRequestServer) (*pb.WriteReplyServer, error) {
	log.Printf("Received: %v", string(in.GetData()))
	log.Printf("Received: %v", string(in.GetSignature()))
	replyHash := []byte("PathHash sent from server")
	return &pb.WriteReplyServer{PathHash: replyHash}, nil
}

func main() {
	flag.Parse()
	lis, err := net.Listen("tcp", fmt.Sprintf(":%d", *port))
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	s := grpc.NewServer()
	pb.RegisterWriteServerServer(s, &server{})
	log.Printf("server listening at %v", lis.Addr())
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}

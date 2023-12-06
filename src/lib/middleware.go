package lib

import (
	"bytes"
	pb "cfs/middleware/src/lib/go_proto"
	"context"
	"crypto/rsa"
	"fmt"
	"github.com/golang/protobuf/proto"
	"time"
)

type MiddlewareServer struct {
	pb.UnimplementedMiddlewareServer

	InodeClient     pb.DataCapsuleClient
	DataClient      pb.DataCapsuleClient
	InodeSigningKey *rsa.PrivateKey
	DataSigningKey  *rsa.PrivateKey
	EnableCrypto    bool
}

func (s *MiddlewareServer) PutINode(ctx context.Context, in *pb.PutINodeRequest) (*pb.PutINodeResponse, error) {
	fmt.Println("Received inode")
	fmt.Println(proto.MarshalTextString(in.Block))
	// 1. validate the client's signature first
	if s.EnableCrypto && !ValidateDataCapsuleFileSystemBlock(in.Block) {
		return &pb.PutINodeResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}
	fmt.Println("Signature check passed")

	// 2. ensure that the client is in ACL of the node attached to
	// todo: edge case: previous node is marked as deleted
	// todo: put a deleted = true node, need to verify leafs
	if s.EnableCrypto && !s.validateFsBlock(in.Block, in.Block.PrevHash) {
		return &pb.PutINodeResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 3. ship the node to the server
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	block := s.finalizeBlock(in.Block, s.InodeSigningKey)
	result, err := s.InodeClient.Put(ctx, &pb.PutRequest{Block: block})
	if err != nil {
		panic(err)
	}

	return &pb.PutINodeResponse{
		Success: result.Success,
		Hash:    &result.Hash,
		Block:   block,
	}, nil
}

func (s *MiddlewareServer) PutData(ctx context.Context, in *pb.PutDataRequest) (*pb.PutDataResponse, error) {
	// 1. validate the client's signature first
	if s.EnableCrypto && !ValidateDataCapsuleFileSystemBlock(in.Block) {
		return &pb.PutDataResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 2. ensure that the client is in ACL of the referenced inode attached to
	// todo: removed users putting junk data
	if s.EnableCrypto && !s.validateFsBlock(in.Block, in.InodeHash) {
		return &pb.PutDataResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 3. ship the node to the server
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	block := s.finalizeBlock(in.Block, s.DataSigningKey)
	result, err := s.DataClient.Put(ctx, &pb.PutRequest{Block: block})
	if err != nil {
		panic(err)
	}

	return &pb.PutDataResponse{
		Success: result.Success,
		Hash:    &result.Hash,
		Block:   block,
	}, nil
}

func (s *MiddlewareServer) finalizeBlock(fsBlock *pb.DataCapsuleFileSystemBlock, key *rsa.PrivateKey) *pb.DataCapsuleBlock {
	finalizedBlock := pb.DataCapsuleBlock{
		PrevHash:  fsBlock.PrevHash,
		Fs:        fsBlock,
		Timestamp: time.Now().UnixNano(),
		Signature: []byte{},
	}

	if s.EnableCrypto {
		SignDataCapsuleBlock(&finalizedBlock, key)
	}

	return &finalizedBlock
}

func (s *MiddlewareServer) validateFsBlock(fsBlock *pb.DataCapsuleFileSystemBlock, prevHash string) bool {
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	prevBlock, err := s.InodeClient.Get(ctx, &pb.GetRequest{BlockHash: prevHash})
	if err != nil {
		panic(err)
	}

	found := false
	for _, prevId := range prevBlock.Block.Fs.GetInode().WriteAllowList {
		if bytes.Equal(prevId.PubKey, fsBlock.UpdatedBy.PubKey) && prevId.Uid == fsBlock.UpdatedBy.Uid && ValidateID(prevId) {
			found = true
			break
		}
	}
	return found
}

package lib

import (
	"bytes"
	"context"
	"crypto/rsa"
	pb "dcfs2/middleware/src/lib/go_proto"
	"time"
)

type MiddlewareServer struct {
	pb.UnimplementedMiddlewareServer

	InodeClient pb.DataCapsuleClient
	DataClient  pb.DataCapsuleClient
	PrivateKey  *rsa.PrivateKey
}

func (s *MiddlewareServer) PutINode(ctx context.Context, in *pb.PutINodeRequest) (*pb.PutINodeResponse, error) {
	// 1. validate the client's signature first
	if !ValidateDataCapsuleFileSystemBlock(in.Block) {
		return &pb.PutINodeResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 2. ensure that the client is in ACL of the node attached to
	// todo: edge case: previous node is marked as deleted
	if !s.validateFsBlock(in.Block, in.Block.PrevHash) {
		return &pb.PutINodeResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 3. ship the node to the server
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	result, err := s.InodeClient.Put(ctx, &pb.PutRequest{Block: s.finalizeBlock(in.Block)})
	if err != nil {
		panic(err)
	}

	return &pb.PutINodeResponse{
		Success: result.Success,
		Hash:    nil,
	}, nil
}

func (s *MiddlewareServer) PutData(ctx context.Context, in *pb.PutDataRequest) (*pb.PutDataResponse, error) {
	// 1. validate the client's signature first
	if !ValidateDataCapsuleFileSystemBlock(in.Block) {
		return &pb.PutDataResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 2. ensure that the client is in ACL of the referenced inode attached to
	// todo: removed users putting junk data
	if !s.validateFsBlock(in.Block, in.InodeHash) {
		return &pb.PutDataResponse{
			Success: false,
			Hash:    nil,
		}, nil
	}

	// 3. ship the node to the server
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	result, err := s.DataClient.Put(ctx, &pb.PutRequest{Block: s.finalizeBlock(in.Block)})
	if err != nil {
		panic(err)
	}

	return &pb.PutDataResponse{
		Success: result.Success,
		Hash:    nil,
	}, nil
}

func (s *MiddlewareServer) finalizeBlock(fsBlock *pb.DataCapsuleFileSystemBlock) *pb.DataCapsuleBlock {
	finalizedBlock := pb.DataCapsuleBlock{
		PrevHash:  fsBlock.PrevHash,
		Fs:        fsBlock,
		Timestamp: 0,
		Signature: nil,
	}
	SignDataCapsuleBlock(&finalizedBlock, s.PrivateKey)

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
		if bytes.Equal(prevId.PubKey, fsBlock.UpdatedBy.PubKey) && bytes.Equal(prevId.Uid, fsBlock.UpdatedBy.PubKey) && ValidateID(prevId) {
			found = true
			break
		}
	}
	return found
}

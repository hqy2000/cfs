package main

import (
	"context"
	"crypto/rsa"
	"crypto/sha256"
	"flag"
	"fmt"
	"log"
	"time"

	middlewareCrypto "dcfs2/middleware/middlewareCrypto"
	pb "dcfs2/middleware/proto"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func GenerateDummyDataAndSignature() ([]byte, []byte) {
	return []byte("Data Sent From Middleware"), []byte("Dummy Signature")
}

func SignWriteRequest(data []byte, privateKey rsa.PrivateKey) []byte {
	h := sha256.New()
	h.Write(data)
	hashSum := h.Sum(nil)
	signature, _ := middlewareCrypto.SignData(hashSum, privateKey)
	return signature
}

func main() {

	// Placeholder, user will provide its public key and keep their private key in secret

	userPrivateKey, userPublicKey := middlewareCrypto.GenerateRSAKey(2048)
	secretData := "UserData"
	fmt.Println("CHECKPOINT")
	encryptedData := middlewareCrypto.EncryptOAEP(secretData, userPublicKey)
	middlewareCrypto.DecryptOAEP(encryptedData, *userPrivateKey)
	// Now that we ha ve the encryptedData we can send it to server

	// Experimenting communication with server
	dummyData, _ := GenerateDummyDataAndSignature()
	dummySignature := SignWriteRequest(dummyData, *userPrivateKey)

	addr := flag.String("addr", "localhost:50051", "the address to connect to")
	conn, err := grpc.Dial(*addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatalf("did not connect: %v", err)
	}
	defer conn.Close()
	c := pb.NewMiddlewareClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	r, err := c.Put(ctx, &pb.PutMiddlewareRequest{Data: dummyData, Signature: dummySignature, Timestamp: time.Now().Unix()})
	if err != nil {
		log.Fatalf("could not greet: %v", err)
	}
	pathHash := r.GetHash()
	log.Printf("Receiving: %s", pathHash)

}

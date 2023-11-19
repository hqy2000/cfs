package lib

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"dcfs2/middleware/src/lib/go_proto"
	"encoding/base64"
	"encoding/pem"
	"fmt"
	"github.com/golang/protobuf/proto"
	"os"
	"time"
)

func SignDataCapsuleBlock(block *go_proto.DataCapsuleBlock, privateKey *rsa.PrivateKey) {
	block.Signature = []byte{}
	block.Timestamp = time.Now().UnixNano()
	block.Signature = SignData(block, privateKey)
}

func SignData(data proto.Message, privateKey *rsa.PrivateKey) []byte {
	bytes, err := proto.Marshal(data)
	if err != nil {
		panic(err)
	}
	hashed := sha256.Sum256(bytes)
	signature, err := rsa.SignPKCS1v15(nil, privateKey, crypto.SHA256, hashed[:])
	if err != nil {
		panic(err)
	}

	return signature
}

func ValidateDataCapsuleFileSystemBlock(block *go_proto.DataCapsuleFileSystemBlock) bool {
	if !ValidateID(block.UpdatedBy) {
		return false
	}
	signature := block.Signature
	block.Signature = []byte{}

	result := ValidateData(block, LoadPublicKey(block.UpdatedBy.PubKey), signature)
	block.Signature = signature

	return result
}

func ValidateID(id *go_proto.ID) bool {
	signature := id.Signature
	id.Signature = []byte{}

	result := ValidateData(id, LoadPublicKey(id.PubKey), signature)
	id.Signature = signature

	return result
}

func ValidateData(data proto.Message, publicKey *rsa.PublicKey, signature []byte) bool {
	bytes, err := proto.Marshal(data)
	if err != nil {
		panic(err)
	}

	hashed := sha256.Sum256(bytes)
	result := rsa.VerifyPKCS1v15(publicKey, crypto.SHA256, hashed[:], signature)

	return result == nil
}

func LoadPublicKey(key []byte) *rsa.PublicKey {
	block, _ := pem.Decode(key)
	if block == nil {
		panic("failed to parse PEM block containing the key")
	}

	pub, err := x509.ParsePKIXPublicKey(block.Bytes)
	if err != nil {
		panic(err)
	}

	switch pub := pub.(type) {
	case *rsa.PublicKey:
		return pub
	}

	panic("Key type is not RSA")
}

func EncryptOAEPData(secretMessage []byte, pubkey rsa.PublicKey) []byte {
	label := []byte("OAEP Encrypted")
	// crypto/rand.Reader is a good source of entropy for randomizing the
	// encryption function.
	rng := rand.Reader
	ciphertext, err := rsa.EncryptOAEP(sha256.New(), rng, &pubkey, secretMessage, label)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error from encryption: %s\n", err)
		return nil
	}
	return ciphertext
}

func DecryptOAEPData(cipherText []byte, privKey rsa.PrivateKey) []byte {
	label := []byte("OAEP Encrypted")

	// crypto/rand.Reader is a good source of entropy for blinding the RSA
	// operation.
	rng := rand.Reader
	plaintext, err := rsa.DecryptOAEP(sha256.New(), rng, &privKey, cipherText, label)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error from decryption: %s\n", err)
		return nil
	}
	fmt.Printf("Plaintext: %s\n", string(plaintext))

	return plaintext
}

func EncryptOAEP(secretMessage string, pubkey rsa.PublicKey) string {
	ciphertext := EncryptOAEPData([]byte(secretMessage), pubkey)
	return base64.StdEncoding.EncodeToString(ciphertext)
}

func DecryptOAEP(cipherText string, privKey rsa.PrivateKey) string {
	ct, _ := base64.StdEncoding.DecodeString(cipherText)
	plaintext := DecryptOAEPData(ct, privKey)
	return string(plaintext)
}

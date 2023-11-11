package main

import (
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"os"
)

func EncryptOAEP(secretMessage string, pubkey rsa.PublicKey) string {
	label := []byte("OAEP Encrypted")
	// crypto/rand.Reader is a good source of entropy for randomizing the
	// encryption function.
	rng := rand.Reader
	ciphertext, err := rsa.EncryptOAEP(sha256.New(), rng, &pubkey, []byte(secretMessage), label)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error from encryption: %s\n", err)
		return "Error from encryption"
	}
	return base64.StdEncoding.EncodeToString(ciphertext)
}

func DecryptOAEP(cipherText string, privKey rsa.PrivateKey) string {
	ct, _ := base64.StdEncoding.DecodeString(cipherText)
	label := []byte("OAEP Encrypted")

	// crypto/rand.Reader is a good source of entropy for blinding the RSA
	// operation.
	rng := rand.Reader
	plaintext, err := rsa.DecryptOAEP(sha256.New(), rng, &privKey, ct, label)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error from decryption: %s\n", err)
		return "Error from Decryption"
	}
	fmt.Printf("Plaintext: %s\n", string(plaintext))

	return string(plaintext)
}

func main() {

	// Placeholder, user will provide its public key and keep their private key in secret

	userPrivateKey, userErr := rsa.GenerateKey(rand.Reader, 2048)
	if userErr != nil {
		fmt.Println(userErr.Error)
		os.Exit(1)
	}
	userPublicKey := userPrivateKey.PublicKey
	secretData := "UserData"
	fmt.Println("CHECKPOINT")
	encryptedData := EncryptOAEP(secretData, userPublicKey)
	DecryptOAEP(encryptedData, *userPrivateKey)
	// Now that we ha ve the encryptedData we can send it to server

}

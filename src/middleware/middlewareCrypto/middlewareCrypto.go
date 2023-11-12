package crypto

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"encoding/base64"
	"fmt"
	"os"
)

func GenerateRSAKey(bits int) (*rsa.PrivateKey, rsa.PublicKey) {
	userPrivateKey, userErr := rsa.GenerateKey(rand.Reader, bits)
	if userErr != nil {
		fmt.Println(userErr.Error)
		os.Exit(1)
	}
	userPublicKey := userPrivateKey.PublicKey
	return userPrivateKey, userPublicKey
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

func SignData(data []byte, privateKey rsa.PrivateKey) ([]byte, error) {
	hashed := sha256.Sum256(data)
	signature, err := rsa.SignPKCS1v15(nil, &privateKey, crypto.SHA256, hashed[:])
	if err != nil {
		return nil, err
	}
	return signature, nil
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

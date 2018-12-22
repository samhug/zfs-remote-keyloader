package main

import (
	"bytes"
	"fmt"
	"log"
	"os"
	"os/exec"
)

func usage() {
	fmt.Println("Usage: test-init-app")
}

func main() {

	if len(os.Args) > 1 && os.Args[1] == "--help" {
		usage()
		return
	}

	fmt.Println("Hello from the golang test init app")

	cmd := exec.Command("zfs", "load-key", "-L", "file:///test-key", "rpool/home2")

	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Run(); err != nil {
		fmt.Printf("Err.\n%q\n%q\n", stdout.String(), stderr.String())
		log.Fatal(err)
	}

	fmt.Printf("Done.\n%q\n%q\n", stdout.String(), stderr.String())
}

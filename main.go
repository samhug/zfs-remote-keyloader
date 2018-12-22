package main

import (
	"bytes"
	"fmt"
	"io/ioutil"
	"net"
	"os"
	"os/exec"
)

func usage() {
	fmt.Println("Usage: test-init-app")
}

type Config struct {
	ListenAddr string
	ListenPort int

	PoolName string
}

func getConfig() *Config {


	if len(os.Args) > 1 && os.Args[1] == "--help" {
		usage()
		os.Exit(0)
	}

	cfg := &Config{}

	cfg.ListenAddr = "0.0.0.0"
	cfg.ListenPort = 3333
	cfg.PoolName = "rpool/home2"

	return cfg
}

func getIfaceList() ([]string, error) {
	ifaces, err := net.Interfaces()
	if err != nil {
		return nil, err
	}
	ips := []string{}
	for _, iface := range ifaces {
		addrs, err := iface.Addrs()
		if err != nil {
			return nil, err
		}
		for _, addr := range addrs {
			var ip net.IP
			switch v := addr.(type) {
			case *net.IPNet:
				ip = v.IP
			case *net.IPAddr:
				ip = v.IP
			}
			ips = append(ips, ip.String())
		}
	}
	return ips, nil
}

func main() {

	cfg := getConfig()

	fmt.Println("Hello from the golang test init app")

	ips, err := getIfaceList()
	for _, ip := range ips {
		fmt.Println(ip)
	}

	l, err := net.Listen("tcp", fmt.Sprintf("%s:%d", cfg.ListenAddr, cfg.ListenPort))
	if err != nil {
		fmt.Println("Error listening:", err.Error())
		os.Exit(1)
	}
	defer l.Close()

	for {
		conn, err := l.Accept()
		if err != nil {
			fmt.Println("Error accepting: ", err.Error())
			continue
		}

		go handleRequest(conn, cfg)
	}
}

func handleRequest(conn net.Conn, cfg *Config) {

	defer conn.Close()

	keyBuf := make([]byte, 512)

	keyLen, err := conn.Read(keyBuf)
	if err != nil {
		fmt.Println("Error reading", err.Error())
	}

	conn.Write([]byte("Attempting to load zfs key:\n"))

	f, err := ioutil.TempFile("/tmp", "zfs-key")
	if err != nil {
		fmt.Println("Error creating key file: ", err.Error())
		return
	}

	defer os.Remove(f.Name())

	key := keyBuf[:keyLen]

	if _, err := f.Write(key); err != nil {
		fmt.Println("Error writing to key file: ", err.Error())
		return
	}
	if err := f.Close(); err != nil {
		fmt.Println("Error closing key file: ", err.Error())
		return
	}

	cmd := exec.Command("zfs", "load-key", "-L", fmt.Sprintf("file://%s", f.Name()), cfg.PoolName)

	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Run(); err != nil {
		conn.Write([]byte(fmt.Sprintf("Error loading zfs key with key='%q': \n", key)))
		conn.Write(stdout.Bytes())
		conn.Write(stderr.Bytes())
		return
	}

	conn.Write(stdout.Bytes())
	conn.Write(stderr.Bytes())


	// we've successfully unlocked, shutdown the server
	os.Exit(0)
}

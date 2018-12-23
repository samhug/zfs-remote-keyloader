package main

import (
	"fmt"
	"html/template"
	"io/ioutil"
	"net"
	"net/http"
	"os"
	"os/exec"
)

func usage() {
	fmt.Println("Usage: test-init-app")
}

type Config struct {
	ListenAddr string
	ListenPort int
	PoolName   string
}

type tmplData struct {
	Success bool
	FSName  string
	Message string
	Output  string
}

func loadConfig() {

	if len(os.Args) > 1 && os.Args[1] == "--help" {
		usage()
		os.Exit(0)
	}

	config = &Config{}

	config.ListenAddr = "0.0.0.0"
	config.ListenPort = 3333
	config.PoolName = "rpool/home2"
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

var config *Config
var server *http.Server
var responseTmpl *template.Template

func main() {

	loadConfig()
	loadTemplates()

	fmt.Println("Hello from the golang test init app")

	ips, err := getIfaceList()
	if err != nil {
		fmt.Println("Failed to enumerate ip addresses: ", err.Error())
		os.Exit(1)
	}

	for _, ip := range ips {
		fmt.Println(ip)
	}

	http.HandleFunc("/", handleRequest)

	server = &http.Server{
		Addr: fmt.Sprintf("%s:%d", config.ListenAddr, config.ListenPort),
	}

	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		fmt.Println("HTTP Serve Error: ", err.Error())
		os.Exit(1)
	}
}

func handleRequest(w http.ResponseWriter, r *http.Request) {

	if r.Method != http.MethodPost {
		data := &tmplData{
			FSName: config.PoolName,
		}
		responseTmpl.Execute(w, data)
		return
	}

	key := []byte(r.FormValue("decryption-key"))

	output, err := zfsLoadKey(key, config.PoolName)
	if err != nil {
		data := &tmplData{
			Success: false,
			FSName:  config.PoolName,
			Message: fmt.Sprintf("Failed: %s", err.Error()),
			Output:  output,
		}
		responseTmpl.Execute(w, data)

		return
	}

	data := &tmplData{
		Success: true,
		Message: "Success!",
		Output:  output,
	}
	responseTmpl.Execute(w, data)

	// we've successfully unlocked, shutdown the server
	go server.Shutdown(nil)
}

func zfsLoadKey(key []byte, fsName string) (string, error) {
	f, err := ioutil.TempFile("/tmp", "zfs-key")
	if err != nil {
		return "", fmt.Errorf("Error creating key file: %s", err.Error())
	}
	defer os.Remove(f.Name())

	if _, err := f.Write(key); err != nil {
		return "", fmt.Errorf("Error writing to key file: %s", err.Error())
	}
	if err := f.Close(); err != nil {
		return "", fmt.Errorf("Error closing key file: %s", err.Error())
	}

	cmd := exec.Command("zfs", "load-key", "-L", fmt.Sprintf("file://%s", f.Name()), fsName)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return string(output), fmt.Errorf("Error loading zfs key: %s", err.Error())
	}

	return string(output), nil
}

func loadTemplates() {
	var err error

	responseTmplStr := `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>Server remote disk decrypt</title>
</head>
<body>
<h2>{{.Message}}</h2>
{{if .Success}}
<p>{{.Output}}</p>
{{else}}
<form method="POST">
<label>Enter decryption key for "{{.FSName}}":</label><br />
<input type="password" name="decryption-key"><br />
<input type="submit">
</form>
{{end}}
</body>
</html>
`

	if responseTmpl, err = template.New("response").Parse(responseTmplStr); err != nil {
		panic(err)
	}
}

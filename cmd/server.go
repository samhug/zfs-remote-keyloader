package cmd

import (
	"context"
	"fmt"

	"github.com/samhug/zfs-remote-keyloader/zfs"
	"github.com/spf13/cobra"

	"html/template"
	"log"
	"net"
	"net/http"
)

var listenAddr string
var datasetName string

var responseTmpl *template.Template

func init() {
	rootCmd.AddCommand(serverCmd)

	initTemplates()

	serverCmd.Flags().StringVar(&listenAddr, "listen", "0.0.0.0:3333", "addr:port to listen on")
	serverCmd.Flags().StringVar(&datasetName, "dataset", "", "ZFS dataset to load keys for")
	serverCmd.MarkFlagRequired("dataset")
}

var serverCmd = &cobra.Command{
	Use:   "server",
	Short: "Start HTTP remote key load server",
	Long:  `Serves a web form over HTTP to prompt for ZFS dataset decryption keys`,
	Run:   serverMain,
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

var server *http.Server
var exitCh chan bool

func serverMain(cmd *cobra.Command, args []string) {

	log.Println("starting zfs-remote-keyloader server...")

	ips, err := getIfaceList()
	if err != nil {
		log.Fatalln("Failed to enumerate ip addresses: ", err.Error())
	}

	fmt.Printf("Server has %d IP(s):\n", len(ips))
	for _, ip := range ips {
		fmt.Println(" -", ip)
	}

	log.Println("listening at ", listenAddr)

	http.HandleFunc("/", handleRequest)

	server = &http.Server{
		Addr: listenAddr,
	}
	exitCh = make(chan bool)

	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatal(err)
		}
	}()

	// Wait for the signal to shutdown the server
	select {
	case <-exitCh:
		server.Shutdown(context.Background())
	}

	log.Println("Finished")
}

func handleRequest(w http.ResponseWriter, r *http.Request) {

	if r.Method != http.MethodPost {
		data := &responseTmplData{
			DatasetName: datasetName,
		}
		responseTmpl.Execute(w, data)
		return
	}

	key := []byte(r.FormValue("decryption-key"))

	if err := zfs.LoadKey(key, datasetName); err != nil {
		data := &responseTmplData{
			Success:     false,
			DatasetName: datasetName,
			Message:     fmt.Sprintf("Failed: %s", err.Error()),
		}
		responseTmpl.Execute(w, data)
		return
	}

	data := &responseTmplData{
		Success: true,
		Message: "Success!",
	}
	responseTmpl.Execute(w, data)

	// We've successfully unlocked, send the signal to shutdown the server
	exitCh <- true
}

type responseTmplData struct {
	Success     bool
	DatasetName string
	Message     string
}

func initTemplates() {

	responseTmplStr := `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>ZFS Remote Key Loader</title>
</head>
<body>
<h2>{{.Message}}</h2>
{{if .Success}}
{{else}}
<form method="POST">
<label>Enter decryption key for "{{.DatasetName}}":</label><br />
<input type="password" name="decryption-key"><br />
<input type="submit">
</form>
{{end}}
</body>
</html>
`

	responseTmpl = template.Must(template.New("response").Parse(responseTmplStr))
}

package zfs

import (
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
)

func LoadKey(key []byte, fsName string) error {

	// Create a temporary file to hold the key material
	f, err := ioutil.TempFile("/tmp", "zfs-key")
	if err != nil {
		return fmt.Errorf("Error creating temp key file: %s", err.Error())
	}
	defer os.Remove(f.Name())

	// Save the key material
	if _, err := f.Write(key); err != nil {
		return fmt.Errorf("Error writing to key file: %s", err.Error())
	}
	if err := f.Close(); err != nil {
		return fmt.Errorf("Error closing key file: %s", err.Error())
	}

	// Run the zfs command
	cmd := exec.Command("zfs", "load-key", "-L", fmt.Sprintf("file://%s", f.Name()), fsName)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("Error loading zfs key: %s", err.Error())
	}

	return nil
}

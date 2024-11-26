/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

// Package agentclient connection between agent and server
package agentclient

import (
	"context"
	"fmt"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/backoff"
	"google.golang.org/protobuf/types/known/structpb"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

// Client defines the client stub for OS service
type Client struct {
	sockAddr string
	client   pb.OSClient
}

// DownloadInfo contains the information required for image download
type DownloadInfo struct {
	ImageURL       string
	FlagSafe       bool
	CheckSum       string
	CaCert         string
	ClientCert     string
	ClientKey      string
	MTLS           bool
	ImageType      string
	ContainerImage string
}

// ConfigsInfo contains the information required for settings
type ConfigsInfo struct {
	Configs []SysConfig
}

// SysConfig contains configuration parameters of a type
type SysConfig struct {
	Model      string
	ConfigPath string
	Contents   map[string]KeyInfo
}

// KeyInfo contains value and operation (i.e. delete, update or add) of a given key for configuration
type KeyInfo struct {
	Value     interface{}
	Operation string
}

// New create a gRPC channel to communicate with the server and return a client stub to perform RPCs
func New(sockAddr string) (*Client, error) {
	if sockAddr == "" {
		return nil, fmt.Errorf("sock addr is empty")
	}

	defaultTimeout := 3 * time.Second
	ctx, cancel := context.WithTimeout(context.Background(), defaultTimeout)
	defer cancel()

	bc := backoff.DefaultConfig
	bc.MaxDelay = defaultTimeout
	conn, err := grpc.DialContext(ctx, sockAddr, grpc.WithInsecure(), grpc.WithBlock(),
		grpc.WithConnectParams(grpc.ConnectParams{Backoff: bc}))
	if err != nil {
		return nil, err
	}
	return &Client{sockAddr: sockAddr, client: pb.NewOSClient(conn)}, nil
}

// UpdateSpec send update requests to the server in os-agent
func (c *Client) UpdateSpec(version string, downloadInfo *DownloadInfo) error {
	certs := &pb.CertsInfo{
		CaCaert:    downloadInfo.CaCert,
		ClientCert: downloadInfo.ClientCert,
		ClientKey:  downloadInfo.ClientKey,
	}
	_, err := c.client.Update(context.Background(),
		&pb.UpdateRequest{
			Version:        version,
			ImageUrl:       downloadInfo.ImageURL,
			FlagSafe:       downloadInfo.FlagSafe,
			CheckSum:       downloadInfo.CheckSum,
			MTLS:           downloadInfo.MTLS,
			Certs:          certs,
			ImageType:      downloadInfo.ImageType,
			ContainerImage: downloadInfo.ContainerImage,
		})
	return err
}

// RollbackSpec send rollback requests to the server in os-agent
func (c *Client) RollbackSpec() error {
	_, err := c.client.Rollback(context.Background(), &pb.RollbackRequest{})
	return err
}

// ConfigureSpec send configure requests to the server in os-agent
func (c *Client) ConfigureSpec(configsInfo *ConfigsInfo) error {
	var sysConfigs []*pb.SysConfig
	configs := configsInfo.Configs
	for _, config := range configs {
		sysConfig := &pb.SysConfig{
			Model:      config.Model,
			ConfigPath: config.ConfigPath,
		}
		sysContents := make(map[string]*pb.KeyInfo)
		for configName, content := range config.Contents {
			structValue, err := structpb.NewValue(content.Value)
			if err != nil {
				return err
			}
			sysContents[configName] = &pb.KeyInfo{
				Value:     structValue.GetStructValue(),
				Operation: content.Operation,
			}
		}
		sysConfig.Contents = sysContents
		sysConfigs = append(sysConfigs, sysConfig)
	}
	_, err := c.client.Configure(context.Background(), &pb.ConfigureRequest{Configs: sysConfigs})
	return err
}

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

	pb "openeuler.org/saiyan/cmd/agent/api"
)

// Client defines the client stub for OS service
type Client struct {
	sockAddr string
	client   pb.OSClient
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

// UpdateSpec send requests to the server in os-agent
func (c *Client) UpdateSpec(version string, imageURL string, flagSafe bool,checkSum string) error {
	_, err := c.client.Update(context.Background(), &pb.UpdateRequest{Version: version, ImageUrl: imageURL,
		FlagSafe: flagSafe, CheckSum: checkSum})
	return err
}

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

package main

import (
	"fmt"

	"github.com/sirupsen/logrus"
	"google.golang.org/grpc"
	pb "openeuler.org/saiyan/cmd/agent/api"
	"openeuler.org/saiyan/cmd/agent/server"
	"openeuler.org/saiyan/pkg/version"
)

func main() {
	fmt.Println("Version is:", version.Version)
	l, err := server.NewListener(server.SockDir, server.SockName)
	if err != nil {
		logrus.Errorln("listen error" + err.Error())
		return
	}
	s := &server.Server{}
	grpcServer := grpc.NewServer()
	pb.RegisterOSServer(grpcServer, s)
	logrus.Info("os-agent start serving")
	if err := grpcServer.Serve(l); err != nil {
		logrus.Errorln("os-agent serve error" + err.Error())
		return
	}
}

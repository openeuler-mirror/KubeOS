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

// Package internal implements the scripts and invocation of KubeOS image customization.
package internal

import (
	"fmt"

	"github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
)

var cfgFile string

// ApplyUserConfiguration returns a Cobra command for user configuration
func ApplyUserConfiguration() *cobra.Command {
	cmd := &cobra.Command{
		// command name
		Use: "kbimg",
		// command description
		Long: `kbimg is a tool used to create and execute shell scripts by generate kbimg.yaml
		run "kbimg --config" to apply your configurations`,

		// keep silence when the command is wrong
		SilenceUsage: false,

		RunE: func(cmd *cobra.Command, args []string) error {
			return run()
		},

		PreRun: func(cmd *cobra.Command, args []string) {
			if len(args) == 0 && cfgFile == "" {
				cmd.Usage()
			}
		},

		Args: func(cmd *cobra.Command, args []string) error {
			if len(args) > 0 {
				return fmt.Errorf(`run "kbimg --config" to apply your configurations`)
			}
			return nil
		},
	}

	// after --config, applyConfig == true, start generating kbimg.yaml
	cmd.PersistentFlags().StringVarP(&cfgFile, "config", "c", "", "path of the configuration file 'kbimg.yaml'")
	return cmd
}

func run() error {
	if cfgFile != "" {
		logrus.Info("Applying your configurations")
		initConfig()
	}
	return nil
}

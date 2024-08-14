/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2024. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

// Package controllers contains the Reconcile of operator
package controllers

import (
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/selection"
	"openeuler.org/KubeOS/pkg/values"
)

const (
	AllNodeSelector = "all-label"
	NoNodeSelector  = "no-label"
)

type labelRequirementCreator interface {
	createLabelRequirement() ([]labels.Requirement, error)
}

type masterLabel struct {
	op selection.Operator
}

func (ml masterLabel) createLabelRequirement() ([]labels.Requirement, error) {
	requirement, err := labels.NewRequirement(values.LabelMaster, ml.op, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelMaster)
		return []labels.Requirement{}, err
	}
	return []labels.Requirement{*requirement}, nil
}

type opsLabel struct {
	label string
	op    selection.Operator
}

func (ol opsLabel) createLabelRequirement() ([]labels.Requirement, error) {
	requirement, err := labels.NewRequirement(ol.label, ol.op, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+ol.label)
		return []labels.Requirement{}, err
	}
	return []labels.Requirement{*requirement}, nil
}

type nodeSelectorLabel struct {
	value string
	op    selection.Operator
}

func (nl nodeSelectorLabel) createLabelRequirement() ([]labels.Requirement, error) {
	if nl.value == AllNodeSelector {
		return []labels.Requirement{}, nil
	}
	var requirements []labels.Requirement
	// if nodeselector is "", will get the nodes which label value is "",and not have label
	if nl.value == NoNodeSelector {
		requirement, err := labels.NewRequirement(values.LabelNodeSelector, selection.DoesNotExist, nil)
		if err != nil {
			log.Error(err, "unable to create requirement "+values.LabelNodeSelector)
			return []labels.Requirement{}, err
		}
		requirements = append(requirements, *requirement)
		return requirements, nil
	}
	requirement, err := labels.NewRequirement(values.LabelNodeSelector, nl.op, []string{nl.value})
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelNodeSelector)
		return []labels.Requirement{}, err
	}
	requirements = append(requirements, *requirement)
	return requirements, nil
}

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
	"fmt"
	"regexp"
	"time"
)

const (
	DATE_TIME             = "2006-01-02 15:04:05"
	TIME_ONLY             = "15:04:05"
	ExecutionModeSerial   = "serial"
	ExecutionModeParallel = "parallel"
	oneDayTime            = 24 * time.Hour
)

func isWithinTimeWindow(start, end string) (bool, error) {
	if start == "" && end == "" {
		return true, nil
	}
	if start == "" || end == "" {
		return false, fmt.Errorf("invalid TimeWindow: The start time and end time must be both empty or not empty")
	}
	layoutStart, err := checkTimeValid(start)
	if err != nil {
		return false, err
	}
	layoutEnd, err := checkTimeValid(end)
	if err != nil {
		return false, err
	}
	if layoutStart != layoutEnd {
		return false, fmt.Errorf("invalid TimeWindow: Start Time should have same time format with End Time")
	}
	now := time.Now()
	timeFormat := now.Format(layoutStart)
	now, err = time.ParseInLocation(layoutStart, timeFormat, now.Location())
	startTime, err := time.ParseInLocation(layoutStart, start, now.Location())
	if err != nil {
		return false, err
	}
	endTime, err := time.ParseInLocation(layoutStart, end, now.Location())
	if err != nil {
		return false, err
	}
	if endTime.Equal(startTime) {
		return false, fmt.Errorf("invalid TimeWindow: start time is equal to end time")
	}
	if endTime.Before(startTime) {
		if layoutStart == DATE_TIME {
			return false, fmt.Errorf("invalid TimeWindow: Start %s Time is after end time %s",
				startTime.Format(layoutStart), endTime.Format(layoutEnd))
		}
		endTime = endTime.Add(oneDayTime)
		fmt.Printf("endtime time add 24 hour is %s\n", endTime.Format(layoutStart))
		if now.Before(startTime) {
			now = now.Add(oneDayTime)
			fmt.Printf("now time add 24 hour is %s\n", now.Format(layoutStart))
		}

	}
	return now.After(startTime) && now.Before(endTime), nil
}

func checkTimeValid(checkTime string) (string, error) {
	reDateTime, err := regexp.Compile("^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}$")
	if err != nil {
		return "", err
	}
	reTimeOnly, err := regexp.Compile("^\\d{2}:\\d{2}:\\d{2}$")
	if err != nil {
		return "", err
	}

	if reDateTime.MatchString(checkTime) {
		_, err := time.Parse(DATE_TIME, checkTime)
		if err != nil {
			return "", err
		}
		return DATE_TIME, nil

	}
	if reTimeOnly.MatchString(checkTime) {
		_, err := time.Parse(TIME_ONLY, checkTime)
		if err != nil {
			return "", err
		}
		return TIME_ONLY, nil

	}
	return "", fmt.Errorf("invalid TimeWindow: invalid date format, please use date format YYYY-MM-DD HH:MM:SS, or only HH:MM:SS")
}

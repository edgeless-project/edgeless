// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

package main

import (
	"context"
	"os"
	"strconv"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/services/com/api"
)

const measurementType = "com.edgeless.moveRobotArm"

var inst *dda.Dda
var isRoboticArmUp = false

// accepts DDA actions to move the robotic arm
func acceptActions(ctx context.Context) {
	events, err := inst.SubscribeAction(ctx, api.SubscriptionFilter{Type: measurementType})
	if err != nil {
		println("Failed to subscribe to DDA events for arm movement")
	}
	for action := range events {
		println("Received an action to move the robotic arm from DDA")

		time.Sleep(2 * time.Second) // artificial delay

		utf8String := string(action.Params)

		move_diff_value, err := strconv.ParseFloat(utf8String, 64)
		if err != nil {
			println("Failed to parse DDA action params")
			continue
		}

		isRoboticArmUp = move_diff_value > 0
		if isRoboticArmUp {
			println("UP moving - received command from workflow as as temperature was too hot!")
		} else {
			println("DOWN moving - received command from workflow as as temperature was too cold!")
		}

		action.Callback(api.ActionResult{
			Context: "success",
			Data:    []byte("ok"),
		})

	}
}

func main() {
	cfg := config.New()
	cfg.Identity.Name = "mock-robot-arm"
	cfg.Apis.Grpc.Disabled = true
	cfg.Apis.GrpcWeb.Disabled = true
	cfg.Services.Com.Protocol = "mqtt5"
	cfg.Services.Com.Url = os.Getenv("MQTT_DDA")

	instI, err := dda.New(cfg)
	inst = instI
	if err != nil {
		println("something went wrong" + err.Error())
	}

	// open the instance
	open_err := inst.Open(10 * time.Second)
	if open_err != nil {
		println("Could not open DDA instance" + open_err.Error())
	}

	ctx, cancel := context.WithCancel(context.Background())

	println("Starting to accept actions to move my robotic arm from DDA")
	go acceptActions(ctx)

	// This is needed to keep the connection alive
	<-time.After(30 * time.Minute)
	cancel()
}

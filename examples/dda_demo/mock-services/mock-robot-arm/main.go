// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

package main

import (
	"context"
	"os"
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
		println("Failed to subscribe to events for arm movement")
	}
	for action := range events {
		println("received an action to move the robotic arm")

		time.Sleep(2 * time.Second) // artificial delay
		isRoboticArmUp = !isRoboticArmUp
		if isRoboticArmUp {
			println("Beep boop. Robotic arm is now up!")
		} else {
			println("Beep boop. Robotic arm is now down!")
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
	cfg.Cluster = "edgeless-demo"
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
		println("could not open instance" + open_err.Error())
	}

	ctx, cancel := context.WithCancel(context.Background())

	println("starting to accept actions to move my robotic arm")
	go acceptActions(ctx)

	// This is needed to keep the connection alive
	<-time.After(30 * time.Minute)
	cancel()
}

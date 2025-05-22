// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

package main

import (
	"context"
	"net/http"
	"os"
	"strconv"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/services/com/api"
	"github.com/gorilla/websocket"
)

const measurementType = "com.edgeless.moveRobotArm"

var inst *dda.Dda
var isRoboticArmUp = false
var cmds chan string

// for websocket
var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin:     func(r *http.Request) bool { return true }, // allow all origins for now
}

// accepts DDA actions to move the robotic arm
func acceptActions(ctx context.Context) {
	events, err := inst.SubscribeAction(ctx, api.SubscriptionFilter{Type: measurementType})
	if err != nil {
		println("Failed to subscribe to DDA events for arm movement")
	}
	for action := range events {
		println("Received an action to move the robotic arm from DDA")

		time.Sleep(600 * time.Millisecond) // artificial delay

		utf8String := string(action.Params)

		move_diff_value, err := strconv.ParseFloat(utf8String, 64)
		println(move_diff_value)
		if err != nil {
			println("Failed to parse DDA action params")
			continue
		}

		isRoboticArmUp = move_diff_value > 0
		if isRoboticArmUp {
			println("UP moving - received command from workflow as as temperature was too hot!")
			cmds <- "UP"
		} else {
			println("DOWN moving - received command from workflow as as temperature was too cold!")
			cmds <- "DOWN"
		}

		action.Callback(api.ActionResult{
			Context: "success",
			Data:    []byte("ok"),
		})
	}
}

func handleWsConnection(w http.ResponseWriter, r *http.Request) {
	c, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		return
	}
	println("Handling ws connection")

	go func() {
		_, _, err = c.ReadMessage()

		c.SetCloseHandler(func(code int, text string) error {
			println("closing")
			return nil
		})
		println("Relaying commands to the GUI over websocket")
		for cmd := range cmds {
			println("New command: " + cmd)
			c.WriteMessage(websocket.TextMessage, []byte(cmd))
		}
		println("End")
	}()
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

	// prepare for relaying to gui over websocket
	cmds = make(chan string, 100)

	// Start a websockets server which is used by the robotic arm GUI to change
	// its position
	http.HandleFunc("/ws", handleWsConnection)

	// Chrome does not allow ws localhost connections without tls - use firefox
	go http.ListenAndServe(":8019", nil)

	println("Starting to accept actions to move my robotic arm from DDA")
	go acceptActions(ctx)

	// This is needed to keep the connection alive
	<-time.After(30 * time.Minute)
	cancel()
}

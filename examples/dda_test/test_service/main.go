// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

package main

import (
	"context"
	"encoding/binary"
	"fmt"
	"math/rand"
	"strconv"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	com_api "github.com/coatyio/dda/services/com/api"
	"github.com/coatyio/dda/services/state/api"
)

const ddaEventType = "com.dda.event"
const ddaActionType = "com.dda.action"
const keyspace = 100

var inst *dda.Dda

func publishEvents(ctx context.Context) {
	ticker := time.NewTicker(1 * time.Second)
	id := 0
	for range ticker.C {
		data := make([]byte, 8)
		event := com_api.Event{
			Type:   ddaEventType,
			Id:     strconv.Itoa(id),
			Source: (ctx.Value("source")).(string),
			Data:   data,
		}
		err := inst.PublishEvent(event)
		if err != nil {
			println("error publishing event: " + err.Error())
		} else {
			println("publishing event with id=" + strconv.Itoa(id))
		}
		id += 1
	}
}

func subscribeEvents(ctx context.Context) {
	filter := com_api.SubscriptionFilter{
		Type: "com.pub.event",
	}
	events, err := inst.SubscribeEvent(ctx, filter)
	if err != nil {
		println("could not subscribe to events")
	}
	for event := range events {
		println("got an event from edgeless with data=(" + string(event.Data) + ")")
	}
}

func publishActions(ctx context.Context) {
	ticker := time.NewTicker(2 * time.Second)
	id := 0
	for range ticker.C {
		fibNumber := (uint64)(rand.Intn(keyspace))
		bytes := make([]byte, 8)
		binary.BigEndian.PutUint64(bytes, fibNumber)
		action := com_api.Action{
			Type:   ddaActionType,
			Id:     strconv.Itoa(id),
			Source: ctx.Value("source").(string),
			Params: bytes,
		}
		actionCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
		defer cancel()
		results, err := inst.PublishAction(actionCtx, action)
		if err != nil {
			println("Error while publishing an action")
			break
		}
		select {
		case <-actionCtx.Done():
			println("publish action expired")
		case <-results:
			println("Got a result for the action!")
			// since we only want one single result, we can cancel the context
			// already
			cancel()
		}
		id += 1
	}
}

func subscribeAction(ctx context.Context) {
	filter := com_api.SubscriptionFilter{
		Type: "com.pub.action",
	}
	actions, err := inst.SubscribeAction(ctx, filter)
	if err != nil {
		println("could not subscribe to actions")
	}
	for action := range actions {
		println("got an action")
		// respond to an action with some probability - some actions will not
		// receive a response
		if rand.Float32() < 1.0 { // for now all get a response
			// respond
			result := com_api.ActionResult{
				Data: []byte("action result!"),
			}
			err := action.Callback(result)
			if err != nil {
				println("could not send an action result back")
			}
		} else {
			// do nothing -> dda in edgeless should time out
		}
	}

}

func listenToUpdates(ctx context.Context) {
	inputs, err := inst.ObserveStateChange(ctx)
	if err != nil {
		println("can't listen to updates")
		return
	}
	for input := range inputs {
		fmt.Printf("got an input: %s, %s", input.Key, input.Value)
	}
}

func makeUpdates(ctx context.Context) {
	ticker := time.NewTicker(2 * time.Second)
	id := 0
	for range ticker.C {
		input := api.Input{Op: api.InputOpSet, Key: fmt.Sprintf("%s-%d", "hey", id), Value: []byte("world")}
		err := inst.ProposeInput(ctx, &input)
		if err != nil {
			fmt.Printf("error " + err.Error())
		}
		id += 1
	}
}

func main() {
	cfg := config.New()
	cfg.Identity.Name = "dda-test-service"
	cfg.Apis.Grpc.Disabled = true
	cfg.Apis.GrpcWeb.Disabled = true
	cfg.Services.Com.Protocol = "mqtt5"
	cfg.Services.Com.Url = "mqtt://localhost:1883"

	// configure the state service
	cfg.Services.State.Bootstrap = false // the edgeless dda is the bootstrapper for the state binding
	cfg.Services.State.Disabled = false  // enable the state binding

	// create a dda instance
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
	ctx = context.WithValue(ctx, "source", cfg.Identity.Name)

	println("starting the dda-test-service")
	println("its purpose is to test all of the APIs of DDA implemented in Edgeless")
	// cast of dda_com_test
	go publishEvents(ctx)
	go subscribeEvents(ctx)
	// call of dda_com_test
	go publishActions(ctx)
	go subscribeAction(ctx)

	// This is needed to keep the connection alive
	<-time.After(30 * time.Minute)
	cancel()
}

// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

package main

import (
	"context"
	"encoding/binary"
	"fmt"
	"math/rand"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/alexflint/go-arg"
	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/plog"
	com_api "github.com/coatyio/dda/services/com/api"
	"github.com/coatyio/dda/services/state/api"
)

const ddaPubEventType = "com.sub.event"
const ddaPubActionType = "com.sub.action"
const ddaPubQueryType = "com.sub.query"

const ddaSubEventType = "com.pub.event"
const ddaSubActionType = "com.pub.action"
const ddaSubQueryType = "com.pub.query"

const keyspace = 100

const publishInterval = time.Duration(1 * time.Second)
const timeoutDuration = time.Duration(1 * time.Second)

var inst *dda.Dda

func publishEvents(ctx context.Context) {
	ticker := time.NewTicker(publishInterval)
	id := 0
	for range ticker.C {
		data := make([]byte, 8)
		event := com_api.Event{
			Type:   ddaPubEventType,
			Id:     strconv.Itoa(id),
			Source: (ctx.Value("source")).(string),
			Data:   data,
		}
		err := inst.PublishEvent(event)
		if err != nil {
			println("error: " + err.Error())
		} else {
			println("event with id=" + strconv.Itoa(id))
		}
		id += 1
	}
}

func publishActions(ctx context.Context) {
	ticker := time.NewTicker(publishInterval)
	id := 0
	for range ticker.C {
		fibNumber := (uint64)(rand.Intn(keyspace))
		bytes := make([]byte, 8)
		binary.BigEndian.PutUint64(bytes, fibNumber)
		action := com_api.Action{
			Type:   ddaPubActionType,
			Id:     strconv.Itoa(id),
			Source: ctx.Value("source").(string),
			Params: bytes,
		}
		actionCtx, cancel := context.WithTimeout(ctx, timeoutDuration)
		defer cancel()
		results, err := inst.PublishAction(actionCtx, action)
		if err != nil {
			println("error: " + err.Error())
			break
		} else {
			println("action with id=" + strconv.Itoa(id))
		}
		select {
		case <-actionCtx.Done():
			println("timeout")
		case <-results:
			println("ok id=" + strconv.Itoa(id))
			// since we only want one single result, we can cancel the context
			// already
			cancel()
		}
		id += 1
	}
}

func publishQueries(ctx context.Context) {
	ticker := time.NewTicker(publishInterval)
	id := 0
	for range ticker.C {
		fibNumber := (uint64)(rand.Intn(keyspace))
		bytes := make([]byte, 8)
		binary.BigEndian.PutUint64(bytes, fibNumber)
		query := com_api.Query{
			Type:   ddaPubQueryType,
			Id:     strconv.Itoa(id),
			Source: ctx.Value("source").(string),
		}
		actionCtx, cancel := context.WithTimeout(ctx, timeoutDuration)
		defer cancel()
		results, err := inst.PublishQuery(actionCtx, query)
		if err != nil {
			println("error: " + err.Error())
			break
		}
		select {
		case <-actionCtx.Done():
			println("timeout")
		case <-results:
			println("ok id=" + strconv.Itoa(id))
			// since we only want one single result, we can cancel the context
			// already
			cancel()
		}
		id += 1
	}
}

// only events / actions / queries that come from edgeless should be processed
func isFromAgent(source string) bool {
	return strings.Contains(source, "agent")
}

func subscribeEvents(ctx context.Context) {
	filter := com_api.SubscriptionFilter{
		Type: ddaSubEventType,
	}
	events, err := inst.SubscribeEvent(ctx, filter)
	if err != nil {
		println("could not subscribe to events")
	}
	for event := range events {
		if isFromAgent(event.Source) {
			continue
		}
		println("event=(" + string(event.Data) + ")")
	}
}

func subscribeAction(ctx context.Context) {
	filter := com_api.SubscriptionFilter{
		Type: ddaSubActionType,
	}
	actions, err := inst.SubscribeAction(ctx, filter)
	if err != nil {
		println("could not subscribe to actions")
		return
	}
	for action := range actions {
		if isFromAgent(action.Source) {
			continue
		}
		execTime := rand.Intn(100)
		time.Sleep(time.Duration(execTime) * time.Millisecond)

		// respond
		result := com_api.ActionResult{
			Data: []byte("action result!"),
		}
		err := action.Callback(result)
		if err != nil {
			println("could not send an action result back")
		}
	}
}

func subscribeQuery(ctx context.Context) {
	filter := com_api.SubscriptionFilter{
		Type: ddaSubQueryType,
	}
	queries, err := inst.SubscribeQuery(ctx, filter)
	if err != nil {
		println("could not subscribe to queries")
		return
	}
	for query := range queries {
		if isFromAgent(query.Source) {
			continue
		}
		execTime := rand.Intn(100)
		time.Sleep(time.Duration(execTime) * time.Millisecond)

		// respond
		result := com_api.QueryResult{
			Data: []byte("query result!"),
		}
		err := query.Callback(result)
		if err != nil {
			println("could not send a query result back")
		}
	}
}

func publishStateUpdates(ctx context.Context) {
	// state updates should not be published too often
	ticker := time.NewTicker(5 * publishInterval)
	id := 0
	keys := make([]string, 100)
	var input api.Input
	for range ticker.C {
		if rand.Float32() < 0.1 {
			// delete with a small probability
			idx := rand.Intn(len(keys))
			rand_key := keys[idx]
			keys = append(keys[:idx], keys[idx+1:]...)
			input = api.Input{Op: api.InputOpDelete, Key: rand_key, Value: []byte("")}
		} else {
			// with a much higher probability add to the raft state
			key := fmt.Sprintf("%s-%d", "hey", id)
			keys = append(keys, key)
			input = api.Input{Op: api.InputOpSet, Key: key, Value: []byte("dda")}
			id += 1
		}
		err := inst.ProposeInput(ctx, &input)
		if err != nil {
			fmt.Printf("error " + err.Error())
		}
	}
}

func subscribeStateUpdates(ctx context.Context) {
	inputs, err := inst.ObserveStateChange(ctx)
	if err != nil {
		println("can't listen to updates")
		return
	}
	for input := range inputs {
		fmt.Printf("got an input: %s, %s\n", input.Key, input.Value)
	}
}

var args struct {
	Id string `arg:"-i,--id"`
}

func main() {
	// parse the args
	arg.MustParse(&args)
	println("Starting an agent")

	cfg := config.New()
	cfg.Identity.Name = fmt.Sprintf("agent-%s", args.Id)
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
		return
	}

	// open the instance
	open_err := inst.Open(10 * time.Second)
	if open_err != nil {
		println("Could not open DDA instance" + open_err.Error())
		return
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	ctx = context.WithValue(ctx, "source", cfg.Identity.Name)

	// dda_com invocation
	go publishEvents(ctx)
	go publishActions(ctx)
	go publishQueries(ctx)
	go subscribeEvents(ctx)
	go subscribeAction(ctx)
	go subscribeQuery(ctx)

	// dda_state
	go publishStateUpdates(ctx)
	go subscribeStateUpdates(ctx)

	c := make(chan os.Signal, 1)
	// graceful termination
	signal.Notify(c, os.Interrupt, syscall.SIGTERM)
	go func() {
		// Block until a signal is received
		sig := <-c
		inst.Close()
		plog.Printf("Caught signal: %s. Exiting DDA gracefully...\n", sig)
		os.Exit(1)
	}()

	// This is needed to keep the connection alive
	<-time.After(30 * time.Minute)
}

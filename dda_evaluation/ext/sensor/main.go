package main

import (
	"context"
	"strconv"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/services/com/api"
)

func main() {
	cfg := config.New()
	cfg.Identity.Name = "sensor"
	cfg.Apis.Grpc.Disabled = true
	cfg.Apis.GrpcWeb.Disabled = true
	cfg.Services.Com.Protocol = "mqtt5"
	cfg.Services.Com.Url = "mqtt://localhost:1883"

	inst, err := dda.New(cfg)
	if err != nil {
		println("something went wrong" + err.Error())
	}

	open_err := inst.Open(10 * time.Second)
	if open_err != nil {
		println("Could not open DDA instance" + open_err.Error())
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	actions, err := inst.SubscribeAction(ctx, api.SubscriptionFilter{
		Type: "sensor",
	})
	if err != nil {
		panic(err)
	}

	for act := range actions {
		// start a new goroutine for each
		go func() {
			seqNum, err := strconv.ParseInt(string(act.Params), 10, 64)
			if err != nil {
				panic(err)
			}
			time.Sleep(40 * time.Millisecond)
			println("Received sensor action with seqNum:", seqNum)
			act.Callback(api.ActionResult{
				Context: "success",
				Data:    []byte("ok"),
			})
		}()
	}
}

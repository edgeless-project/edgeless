// SPDX-FileCopyrightText: © 2025 Siemens AG
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


var inst *dda.Dda
var statistics = struct {
	actionsReceived int
	waitTimes       []time.Duration
}{}

func acceptActions(ctx context.Context) {
	events, err := inst.SubscribeAction(ctx, api.SubscriptionFilter{Type: "com.actor"})
	if err != nil {
		println("Failed to subscribe to DDA events")
	}
	for action := range events {
		start := time.Now()
		println("Received an action, waiting to respond")

		go func() {

			time.Sleep(100 * time.Millisecond) // artificial delay
			statistics.actionsReceived++
			statistics.waitTimes = append(statistics.waitTimes, time.Since(start))

			action.Callback(api.ActionResult{
				Context: "success",
				Data:    []byte("ok"),
			})
		}()
	}
}

func printStatistics(ctx context.Context, stats *struct {
	actionsReceived int
	waitTimes       []time.Duration
}) {
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()
	
	for {
		select {
		case <-ticker.C:
			println("Statistics:")
			println("Actions received:", stats.actionsReceived)
			println("Average wait time:", averageWaitTime(stats.waitTimes))
			println("min wait time:", func() time.Duration {
				if len(stats.waitTimes) == 0 {
					return 0
				}
				min := stats.waitTimes[0]
				for _, wt := range stats.waitTimes {
					if wt < min {
						min = wt
					}
				}
				return min
			}())
			println("max wait time:", func() time.Duration {
				if len(stats.waitTimes) == 0 {
					return 0
				}
				max := stats.waitTimes[0]
				for _, wt := range stats.waitTimes {
					if wt > max {
						max = wt
					}
				}
				return max
			}())
		case <-ctx.Done():
			return
		}
	}
}

func averageWaitTime(durations []time.Duration) time.Duration {
	if len(durations) == 0 {
		return 0
	}
	var total time.Duration
	for _, d := range durations {
		total += d
	}
	return total / time.Duration(len(durations))
}

func main() {
	dockerFlag := false
	if len(os.Args) > 1 && os.Args[1] == "--docker" {
		dockerFlag = true
	}
	cfg := config.New()
	cfg.Identity.Name = "dda_endpoint"
	cfg.Apis.Grpc.Disabled = false
	cfg.Apis.GrpcWeb.Disabled = true
	cfg.Services.Com.Protocol = "mqtt5"
	if dockerFlag {
		cfg.Services.Com.Url = "mqtt://mqtt:1883"
	} else {
		cfg.Services.Com.Url = "mqtt://localhost:1883"
	}

	// only enable com
	cfg.Services.State.Disabled = true
	cfg.Services.Store.Disabled = true

	instI, err := dda.New(cfg)
	inst = instI
	if err != nil {
		println("something went wrong" + err.Error())
	}

	println("DDA instance created, connecting...")
	// open the instance
	open_err := inst.Open(10 * time.Second)
	if open_err != nil {
		println("Could not open DDA instance" + open_err.Error())
	}

	ctx, _:= context.WithCancel(context.Background())

	go acceptActions(ctx)
	go printStatistics(ctx, &statistics)

	// This is needed to keep the connection alive
	select {}
}

package main

import (
	"context"
	"strconv"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/services/com/api"
)

type Pair struct {
	act   *api.ActionWithCallback
	start *time.Time
}

func main() {
	cfg := config.New()
	cfg.Identity.Name = "sensor"
	cfg.Apis.Grpc.Disabled = true
	cfg.Apis.GrpcWeb.Disabled = true
	cfg.Services.Com.Protocol = "mqtt5"
	cfg.Services.Com.Url = "mqtt://mqtt_broker:1883"

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
		Type: "com.actor",
	})

	if err != nil {
		panic(err)
	}

	answers := make(chan Pair)

	go func() {
		for act := range actions {
			start := time.Now()
			// seqNum, err := strconv.ParseInt(string(act.Params), 10, 64)
			// if err != nil {
			// 	panic(err)
			// }
			timer := time.NewTimer(40 * time.Millisecond)
			go func() {
				<-timer.C
				answers <- Pair{&act, &start}
				// println("sleep," + strconv.Itoa(int(time.Since(start).Milliseconds())))
				// println("Received sensor action with seqNum:", seqNum)
				// println("time," + strconv.Itoa(int(time.Since(start).Milliseconds())))
			}()
		}
	}()

	go func() {
		for pair := range answers {
			go func() {
				println("sleep," + strconv.Itoa(int(time.Since(*pair.start).Milliseconds())))
				pair.act.Callback(api.ActionResult{
					Context: "success",
					Data:    []byte("ok"),
				})
			}()
		}
	}()

	select {}

	// 2nd attempt
	// for {
	// 	select {
	// 	case act := <-actions:
	// 		start := time.Now()
	// 		// seqNum, err := strconv.ParseInt(string(act.Params), 10, 64)
	// 		// if err != nil {
	// 		// 	panic(err)
	// 		// }
	// 		timer := time.NewTimer(40 * time.Millisecond)
	// 		go func() {
	// 			<-timer.C
	// 			answers <- Pair{&act, &start}
	// 			// println("sleep," + strconv.Itoa(int(time.Since(start).Milliseconds())))
	// 			// println("Received sensor action with seqNum:", seqNum)
	// 			// println("time," + strconv.Itoa(int(time.Since(start).Milliseconds())))
	// 		}()
	// 	case pair := <-answers:
	// 		println("sleep," + strconv.Itoa(int(time.Since(*pair.start).Milliseconds())))
	// 		pair.act.Callback(api.ActionResult{
	// 			Context: "success",
	// 			Data:    []byte("ok"),
	// 		})
	// 	}
	// }
}

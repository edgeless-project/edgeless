package main

import (
	"context"
	"fmt"
	"sync/atomic"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/services/com/api"
	"github.com/google/uuid"
)

func main() {
	cfg := config.New()
	cfg.Identity.Name = "tester"
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

	seqNum := atomic.Int64{}

	ticker := time.NewTicker(100 * time.Millisecond)
	for {
		select {
		case <-ticker.C:
			go func() {
				nextNum := seqNum.Add(1)
				results, err := inst.PublishAction(ctx, api.Action{
					Type:   "sensor",
					Id:     uuid.New().String(),
					Source: inst.Identity().Id,
					Params: []byte(fmt.Sprintf(`%d`, nextNum)),
				})
				if err != nil {
					panic(err)
				}
				<-results
				println("Result received, seqNum:", nextNum)
			}()
		case <-ctx.Done():
			panic("this should never happen")
		}
	}
}

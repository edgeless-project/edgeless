package main

import (
	"strconv"
	"time"
)

func main() {
	ticker := time.NewTicker(20 * time.Millisecond)

	answers := make(chan *time.Time)

	go func() {
		for range ticker.C {
			start := time.Now()
			// seqNum, err := strconv.ParseInt(string(act.Params), 10, 64)
			// if err != nil {
			// 	panic(err)
			// }
			timer := time.NewTimer(40 * time.Millisecond)
			go func() {
				<-timer.C
				answers <- &start
				// println("sleep," + strconv.Itoa(int(time.Since(start).Milliseconds())))
				// println("Received sensor action with seqNum:", seqNum)
				// println("time," + strconv.Itoa(int(time.Since(start).Milliseconds())))
			}()
		}
	}()

	go func() {
		for pair := range answers {
			go func() {
				println("sleep," + strconv.Itoa(int(time.Since(*pair).Milliseconds())))
				// pair.act.Callback(api.ActionResult{
				// 	Context: "success",
				// 	Data:    []byte("ok"),
				// })
			}()
		}
	}()

	select {}
	// cfg := config.New()
	// cfg.Identity.Name = "tester"
	// cfg.Apis.Grpc.Disabled = true
	// cfg.Apis.GrpcWeb.Disabled = true
	// cfg.Services.Com.Protocol = "mqtt5"
	// cfg.Services.Com.Url = "mqtt://localhost:1883"

	// inst, err := dda.New(cfg)
	// if err != nil {
	// 	println("something went wrong" + err.Error())
	// }

	// open_err := inst.Open(10 * time.Second)
	// if open_err != nil {
	// 	println("Could not open DDA instance" + open_err.Error())
	// }

	// ctx, cancel := context.WithCancel(context.Background())
	// defer cancel()

	// seqNum := atomic.Int64{}

	// ticker := time.NewTicker(100 * time.Millisecond)
	// for {
	// 	select {
	// 	case <-ticker.C:
	// 		go func() {
	// 			nextNum := seqNum.Add(1)
	// 			results, err := inst.PublishAction(ctx, api.Action{
	// 				Type:   "sensor",
	// 				Id:     uuid.New().String(),
	// 				Source: inst.Identity().Id,
	// 				Params: []byte(fmt.Sprintf(`%d`, nextNum)),
	// 			})
	// 			if err != nil {
	// 				panic(err)
	// 			}
	// 			<-results
	// 			println("Result received, seqNum:", nextNum)
	// 		}()
	// 	case <-ctx.Done():
	// 		panic("this should never happen")
	// 	}
	// }
}

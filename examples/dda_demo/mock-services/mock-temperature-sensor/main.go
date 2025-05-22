// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

package main

import (
	"context"
	"math/rand"
	"os"
	"strconv"
	"time"

	"github.com/coatyio/dda/config"
	"github.com/coatyio/dda/dda"
	"github.com/coatyio/dda/services/com/api"
)

const measurementType = "com.edgeless.temperature"

// const minVal float32 = 0
// const maxVal float32 = 100

var inst *dda.Dda

func publishSensorData() {
	// continuously publish sensor data of a mocked temperature sensor in Celsius
	id := 0
	overheating := false
	minVal := float32(0.0)
	maxVal := float32(40.0)
	periodLength := 0

	for {
		// start a new period if needed
		if periodLength == 0 {
			// flip the switch
			overheating = !overheating
			periodLength = 3 + rand.Intn(7)
		}
		// decrement the period counter
		periodLength -= 1

		if overheating {
			minVal = 60.0
			maxVal = 80.0
		} else {
			minVal = 0.0
			maxVal = 40.0
		}
		randVal := minVal + rand.Float32()*(maxVal-minVal)
		byteArray := []byte(strconv.FormatFloat(float64(randVal), 'f', 2, 32))
		event := api.Event{
			Type:   measurementType,
			Id:     strconv.Itoa(id),
			Source: inst.Identity().Id,
			Data:   byteArray,
		}
		println("Publishing sensor data with id=" + strconv.Itoa(id) + ". The temperature is " + strconv.FormatFloat(float64(randVal), 'f', 2, 32))
		if err := inst.PublishEvent(event); err != nil {
			println("Error publishing event" + err.Error())
		}
		time.Sleep(1000 * time.Millisecond)
		id += 1
	}
}

func main() {
	cfg := config.New()
	cfg.Identity.Name = "mock-temperature-sensor"
	cfg.Apis.Grpc.Disabled = false
	cfg.Apis.GrpcWeb.Disabled = false
	println("grpc: " + cfg.Apis.GrpcWeb.Address)
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

	_, cancel := context.WithCancel(context.Background())

	println("Starting to publish sensor data of type " + measurementType)

	go publishSensorData()

	// This is needed to keep the connection alive
	<-time.After(60 * time.Minute)

	cancel()
}

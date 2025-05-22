import './App.css';
import React, { useState, useEffect } from 'react';
import { ComServiceClient } from './api/com_grpc_web_pb.js';
import GaugeChart from 'react-gauge-chart';
import * as pb from "./api/com_pb";

function App() {
    const [tempValue, setTempValue] = useState(0);

    useEffect(() => {
        const client = new ComServiceClient('http://localhost:8800');
        console.log(client)
        const filter = new pb.SubscriptionFilter().setType("com.edgeless.temperature")
        client.subscribeEvent(filter)
            .on("status", status => console.log("status" + status.details))
            .on("data", result => {
                let byteStr = String.fromCharCode(...result.getData());
                let tempValue = parseFloat(byteStr);
                setTempValue(tempValue / 100)
            })
            .on("end", () => { })
            .on("error", err => {
                console.log("error")
            });
    }, []);

    return (
        <div className="App">
            <header className="App-header">
                <div className="container">
                    <div className="left">0</div>
                    <GaugeChart
                        className="center"
                        id="gauge"
                        percent={tempValue}
                        animateDuration={500}
                        textColor='#000000'
                        hideText={true}
                    />
                    <div className="right">100</div>
                </div>
            </header>
        </div>
    );
}

export default App;

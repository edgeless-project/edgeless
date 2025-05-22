import * as grpcWeb from 'grpc-web';

import * as com_pb from './com_pb';


export class ComServiceClient {
  constructor (hostname: string,
               credentials?: null | { [index: string]: string; },
               options?: null | { [index: string]: any; });

  publishEvent(
    request: com_pb.Event,
    metadata: grpcWeb.Metadata | undefined,
    callback: (err: grpcWeb.RpcError,
               response: com_pb.Ack) => void
  ): grpcWeb.ClientReadableStream<com_pb.Ack>;

  subscribeEvent(
    request: com_pb.SubscriptionFilter,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.Event>;

  publishAction(
    request: com_pb.Action,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.ActionResult>;

  subscribeAction(
    request: com_pb.SubscriptionFilter,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.ActionCorrelated>;

  publishActionResult(
    request: com_pb.ActionResultCorrelated,
    metadata: grpcWeb.Metadata | undefined,
    callback: (err: grpcWeb.RpcError,
               response: com_pb.Ack) => void
  ): grpcWeb.ClientReadableStream<com_pb.Ack>;

  publishQuery(
    request: com_pb.Query,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.QueryResult>;

  subscribeQuery(
    request: com_pb.SubscriptionFilter,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.QueryCorrelated>;

  publishQueryResult(
    request: com_pb.QueryResultCorrelated,
    metadata: grpcWeb.Metadata | undefined,
    callback: (err: grpcWeb.RpcError,
               response: com_pb.Ack) => void
  ): grpcWeb.ClientReadableStream<com_pb.Ack>;

}

export class ComServicePromiseClient {
  constructor (hostname: string,
               credentials?: null | { [index: string]: string; },
               options?: null | { [index: string]: any; });

  publishEvent(
    request: com_pb.Event,
    metadata?: grpcWeb.Metadata
  ): Promise<com_pb.Ack>;

  subscribeEvent(
    request: com_pb.SubscriptionFilter,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.Event>;

  publishAction(
    request: com_pb.Action,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.ActionResult>;

  subscribeAction(
    request: com_pb.SubscriptionFilter,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.ActionCorrelated>;

  publishActionResult(
    request: com_pb.ActionResultCorrelated,
    metadata?: grpcWeb.Metadata
  ): Promise<com_pb.Ack>;

  publishQuery(
    request: com_pb.Query,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.QueryResult>;

  subscribeQuery(
    request: com_pb.SubscriptionFilter,
    metadata?: grpcWeb.Metadata
  ): grpcWeb.ClientReadableStream<com_pb.QueryCorrelated>;

  publishQueryResult(
    request: com_pb.QueryResultCorrelated,
    metadata?: grpcWeb.Metadata
  ): Promise<com_pb.Ack>;

}


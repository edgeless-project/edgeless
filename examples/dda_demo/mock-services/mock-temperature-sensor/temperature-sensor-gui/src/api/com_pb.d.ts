import * as jspb from 'google-protobuf'



export class SubscriptionFilter extends jspb.Message {
  getType(): string;
  setType(value: string): SubscriptionFilter;

  getShare(): string;
  setShare(value: string): SubscriptionFilter;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): SubscriptionFilter.AsObject;
  static toObject(includeInstance: boolean, msg: SubscriptionFilter): SubscriptionFilter.AsObject;
  static serializeBinaryToWriter(message: SubscriptionFilter, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): SubscriptionFilter;
  static deserializeBinaryFromReader(message: SubscriptionFilter, reader: jspb.BinaryReader): SubscriptionFilter;
}

export namespace SubscriptionFilter {
  export type AsObject = {
    type: string,
    share: string,
  }
}

export class Event extends jspb.Message {
  getType(): string;
  setType(value: string): Event;

  getId(): string;
  setId(value: string): Event;

  getSource(): string;
  setSource(value: string): Event;

  getTime(): string;
  setTime(value: string): Event;

  getData(): Uint8Array | string;
  getData_asU8(): Uint8Array;
  getData_asB64(): string;
  setData(value: Uint8Array | string): Event;

  getDataContentType(): string;
  setDataContentType(value: string): Event;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Event.AsObject;
  static toObject(includeInstance: boolean, msg: Event): Event.AsObject;
  static serializeBinaryToWriter(message: Event, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Event;
  static deserializeBinaryFromReader(message: Event, reader: jspb.BinaryReader): Event;
}

export namespace Event {
  export type AsObject = {
    type: string,
    id: string,
    source: string,
    time: string,
    data: Uint8Array | string,
    dataContentType: string,
  }
}

export class Ack extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Ack.AsObject;
  static toObject(includeInstance: boolean, msg: Ack): Ack.AsObject;
  static serializeBinaryToWriter(message: Ack, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Ack;
  static deserializeBinaryFromReader(message: Ack, reader: jspb.BinaryReader): Ack;
}

export namespace Ack {
  export type AsObject = {
  }
}

export class Action extends jspb.Message {
  getType(): string;
  setType(value: string): Action;

  getId(): string;
  setId(value: string): Action;

  getSource(): string;
  setSource(value: string): Action;

  getParams(): Uint8Array | string;
  getParams_asU8(): Uint8Array;
  getParams_asB64(): string;
  setParams(value: Uint8Array | string): Action;

  getDataContentType(): string;
  setDataContentType(value: string): Action;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Action.AsObject;
  static toObject(includeInstance: boolean, msg: Action): Action.AsObject;
  static serializeBinaryToWriter(message: Action, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Action;
  static deserializeBinaryFromReader(message: Action, reader: jspb.BinaryReader): Action;
}

export namespace Action {
  export type AsObject = {
    type: string,
    id: string,
    source: string,
    params: Uint8Array | string,
    dataContentType: string,
  }
}

export class ActionResult extends jspb.Message {
  getContext(): string;
  setContext(value: string): ActionResult;

  getData(): Uint8Array | string;
  getData_asU8(): Uint8Array;
  getData_asB64(): string;
  setData(value: Uint8Array | string): ActionResult;

  getDataContentType(): string;
  setDataContentType(value: string): ActionResult;

  getSequenceNumber(): number;
  setSequenceNumber(value: number): ActionResult;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): ActionResult.AsObject;
  static toObject(includeInstance: boolean, msg: ActionResult): ActionResult.AsObject;
  static serializeBinaryToWriter(message: ActionResult, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): ActionResult;
  static deserializeBinaryFromReader(message: ActionResult, reader: jspb.BinaryReader): ActionResult;
}

export namespace ActionResult {
  export type AsObject = {
    context: string,
    data: Uint8Array | string,
    dataContentType: string,
    sequenceNumber: number,
  }
}

export class ActionCorrelated extends jspb.Message {
  getAction(): Action | undefined;
  setAction(value?: Action): ActionCorrelated;
  hasAction(): boolean;
  clearAction(): ActionCorrelated;

  getCorrelationId(): string;
  setCorrelationId(value: string): ActionCorrelated;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): ActionCorrelated.AsObject;
  static toObject(includeInstance: boolean, msg: ActionCorrelated): ActionCorrelated.AsObject;
  static serializeBinaryToWriter(message: ActionCorrelated, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): ActionCorrelated;
  static deserializeBinaryFromReader(message: ActionCorrelated, reader: jspb.BinaryReader): ActionCorrelated;
}

export namespace ActionCorrelated {
  export type AsObject = {
    action?: Action.AsObject,
    correlationId: string,
  }
}

export class ActionResultCorrelated extends jspb.Message {
  getResult(): ActionResult | undefined;
  setResult(value?: ActionResult): ActionResultCorrelated;
  hasResult(): boolean;
  clearResult(): ActionResultCorrelated;

  getCorrelationId(): string;
  setCorrelationId(value: string): ActionResultCorrelated;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): ActionResultCorrelated.AsObject;
  static toObject(includeInstance: boolean, msg: ActionResultCorrelated): ActionResultCorrelated.AsObject;
  static serializeBinaryToWriter(message: ActionResultCorrelated, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): ActionResultCorrelated;
  static deserializeBinaryFromReader(message: ActionResultCorrelated, reader: jspb.BinaryReader): ActionResultCorrelated;
}

export namespace ActionResultCorrelated {
  export type AsObject = {
    result?: ActionResult.AsObject,
    correlationId: string,
  }
}

export class Query extends jspb.Message {
  getType(): string;
  setType(value: string): Query;

  getId(): string;
  setId(value: string): Query;

  getSource(): string;
  setSource(value: string): Query;

  getData(): Uint8Array | string;
  getData_asU8(): Uint8Array;
  getData_asB64(): string;
  setData(value: Uint8Array | string): Query;

  getDataContentType(): string;
  setDataContentType(value: string): Query;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Query.AsObject;
  static toObject(includeInstance: boolean, msg: Query): Query.AsObject;
  static serializeBinaryToWriter(message: Query, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Query;
  static deserializeBinaryFromReader(message: Query, reader: jspb.BinaryReader): Query;
}

export namespace Query {
  export type AsObject = {
    type: string,
    id: string,
    source: string,
    data: Uint8Array | string,
    dataContentType: string,
  }
}

export class QueryResult extends jspb.Message {
  getContext(): string;
  setContext(value: string): QueryResult;

  getData(): Uint8Array | string;
  getData_asU8(): Uint8Array;
  getData_asB64(): string;
  setData(value: Uint8Array | string): QueryResult;

  getDataContentType(): string;
  setDataContentType(value: string): QueryResult;

  getSequenceNumber(): number;
  setSequenceNumber(value: number): QueryResult;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): QueryResult.AsObject;
  static toObject(includeInstance: boolean, msg: QueryResult): QueryResult.AsObject;
  static serializeBinaryToWriter(message: QueryResult, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): QueryResult;
  static deserializeBinaryFromReader(message: QueryResult, reader: jspb.BinaryReader): QueryResult;
}

export namespace QueryResult {
  export type AsObject = {
    context: string,
    data: Uint8Array | string,
    dataContentType: string,
    sequenceNumber: number,
  }
}

export class QueryCorrelated extends jspb.Message {
  getQuery(): Query | undefined;
  setQuery(value?: Query): QueryCorrelated;
  hasQuery(): boolean;
  clearQuery(): QueryCorrelated;

  getCorrelationId(): string;
  setCorrelationId(value: string): QueryCorrelated;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): QueryCorrelated.AsObject;
  static toObject(includeInstance: boolean, msg: QueryCorrelated): QueryCorrelated.AsObject;
  static serializeBinaryToWriter(message: QueryCorrelated, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): QueryCorrelated;
  static deserializeBinaryFromReader(message: QueryCorrelated, reader: jspb.BinaryReader): QueryCorrelated;
}

export namespace QueryCorrelated {
  export type AsObject = {
    query?: Query.AsObject,
    correlationId: string,
  }
}

export class QueryResultCorrelated extends jspb.Message {
  getResult(): QueryResult | undefined;
  setResult(value?: QueryResult): QueryResultCorrelated;
  hasResult(): boolean;
  clearResult(): QueryResultCorrelated;

  getCorrelationId(): string;
  setCorrelationId(value: string): QueryResultCorrelated;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): QueryResultCorrelated.AsObject;
  static toObject(includeInstance: boolean, msg: QueryResultCorrelated): QueryResultCorrelated.AsObject;
  static serializeBinaryToWriter(message: QueryResultCorrelated, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): QueryResultCorrelated;
  static deserializeBinaryFromReader(message: QueryResultCorrelated, reader: jspb.BinaryReader): QueryResultCorrelated;
}

export namespace QueryResultCorrelated {
  export type AsObject = {
    result?: QueryResult.AsObject,
    correlationId: string,
  }
}


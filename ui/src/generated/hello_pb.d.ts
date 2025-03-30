import * as jspb from 'google-protobuf'



export class HelloRequest extends jspb.Message {
  getName(): string;
  setName(value: string): HelloRequest;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): HelloRequest.AsObject;
  static toObject(includeInstance: boolean, msg: HelloRequest): HelloRequest.AsObject;
  static serializeBinaryToWriter(message: HelloRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): HelloRequest;
  static deserializeBinaryFromReader(message: HelloRequest, reader: jspb.BinaryReader): HelloRequest;
}

export namespace HelloRequest {
  export type AsObject = {
    name: string,
  }
}

export class HelloReply extends jspb.Message {
  getMessage(): string;
  setMessage(value: string): HelloReply;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): HelloReply.AsObject;
  static toObject(includeInstance: boolean, msg: HelloReply): HelloReply.AsObject;
  static serializeBinaryToWriter(message: HelloReply, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): HelloReply;
  static deserializeBinaryFromReader(message: HelloReply, reader: jspb.BinaryReader): HelloReply;
}

export namespace HelloReply {
  export type AsObject = {
    message: string,
  }
}

export class GetMessagesRequest extends jspb.Message {
  getTopic(): string;
  setTopic(value: string): GetMessagesRequest;

  getLimit(): number;
  setLimit(value: number): GetMessagesRequest;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetMessagesRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetMessagesRequest): GetMessagesRequest.AsObject;
  static serializeBinaryToWriter(message: GetMessagesRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetMessagesRequest;
  static deserializeBinaryFromReader(message: GetMessagesRequest, reader: jspb.BinaryReader): GetMessagesRequest;
}

export namespace GetMessagesRequest {
  export type AsObject = {
    topic: string,
    limit: number,
  }
}

export class GetMessagesReply extends jspb.Message {
  getMessagesList(): Array<Message>;
  setMessagesList(value: Array<Message>): GetMessagesReply;
  clearMessagesList(): GetMessagesReply;
  addMessages(value?: Message, index?: number): Message;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetMessagesReply.AsObject;
  static toObject(includeInstance: boolean, msg: GetMessagesReply): GetMessagesReply.AsObject;
  static serializeBinaryToWriter(message: GetMessagesReply, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetMessagesReply;
  static deserializeBinaryFromReader(message: GetMessagesReply, reader: jspb.BinaryReader): GetMessagesReply;
}

export namespace GetMessagesReply {
  export type AsObject = {
    messagesList: Array<Message.AsObject>,
  }
}

export class Message extends jspb.Message {
  getId(): number;
  setId(value: number): Message;

  getTopic(): string;
  setTopic(value: string): Message;

  getPart(): number;
  setPart(value: number): Message;

  getKafkaoffset(): number;
  setKafkaoffset(value: number): Message;

  getPayload(): string;
  setPayload(value: string): Message;

  getCreatedAt(): string;
  setCreatedAt(value: string): Message;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Message.AsObject;
  static toObject(includeInstance: boolean, msg: Message): Message.AsObject;
  static serializeBinaryToWriter(message: Message, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Message;
  static deserializeBinaryFromReader(message: Message, reader: jspb.BinaryReader): Message;
}

export namespace Message {
  export type AsObject = {
    id: number,
    topic: string,
    part: number,
    kafkaoffset: number,
    payload: string,
    createdAt: string,
  }
}


export interface Message {
  id: number;
  topic: string;
  part: number;
  kafkaoffset: number;
  payload: string;
  createdAt: string;
}

export const sayHello = async (name: string): Promise<{message: string}> => {
  try {
    const response = await fetch('http://localhost:8080/hello.HelloApi/SayHello', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/grpc-web+proto',
        'X-Grpc-Web': '1',
      },
      body: encodeRequest({ name }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const arrayBuffer = await response.arrayBuffer();
    const data = decodeResponse(arrayBuffer);
    return { message: data.message };
  } catch (err) {
    console.error('gRPC error:', err);
    throw err;
  }
};

export const getMessages = async (topic: string, limit: number): Promise<{messages: Message[]}> => {
  try {
    const response = await fetch('http://localhost:8080/hello.HelloApi/GetMessages', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/grpc-web+proto',
        'X-Grpc-Web': '1',
      },
      body: encodeRequest({ topic, limit }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const arrayBuffer = await response.arrayBuffer();
    const data = decodeResponse(arrayBuffer);
    return { messages: data.messages || [] };
  } catch (err) {
    console.error('gRPC error:', err);
    throw err;
  }
};

// Helper functions for gRPC-Web binary format
function encodeRequest(obj: any): Uint8Array {
  // Simple JSON encoding for now - in production you'd use protobuf
  const str = JSON.stringify(obj);
  const encoder = new TextEncoder();
  const body = encoder.encode(str);
  
  // Add gRPC-Web frame header (1 byte flags, 4 bytes length)
  const frame = new Uint8Array(5 + body.length);
  frame[0] = 0; // flags (0 = no compression)
  frame[1] = (body.length >>> 24) & 0xFF;
  frame[2] = (body.length >>> 16) & 0xFF;
  frame[3] = (body.length >>> 8) & 0xFF;
  frame[4] = body.length & 0xFF;
  frame.set(body, 5);
  
  return frame;
}

function decodeResponse(arrayBuffer: ArrayBuffer): any {
  // Skip 5 byte gRPC-Web frame header
  const data = new Uint8Array(arrayBuffer).slice(5);
  const decoder = new TextDecoder();
  const str = decoder.decode(data);
  return JSON.parse(str);
}

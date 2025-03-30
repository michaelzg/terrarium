import { useEffect, useRef, useState } from 'react';
import { Box, Paper, Typography, Slider, IconButton, Tooltip } from '@mui/material';
import { Timeline, DataSet, TimelineOptions } from 'vis-timeline/standalone';
import { ZoomIn, ZoomOut } from '@mui/icons-material';
import { getMessages } from '../../services/grpc/client';
import type { Message } from '../../types/proto/hello';
import 'vis-timeline/styles/vis-timeline-graph2d.css';

interface CustomTimelineItem {
  id: number;
  content: string;
  start: Date;
  group: number;
  title?: string;
}

export const MessageTimeline = () => {
  const timelineRef = useRef<HTMLDivElement>(null);
  const [timeline, setTimeline] = useState<Timeline | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [error, setError] = useState('');
  const [limit, setLimit] = useState(50);

  const createTimelineItems = (messages: Message[]): CustomTimelineItem[] => {
    return messages.map((msg) => ({
      id: msg.id,
      content: msg.payload,
      start: new Date(msg.created_at),
      group: msg.part,
      title: `ID: ${msg.id}
Topic: ${msg.topic}
Partition: ${msg.part}
Offset: ${msg.kafkaoffset}
Created: ${msg.created_at}
Payload: ${msg.payload}`
    }));
  };

  useEffect(() => {
    if (timelineRef.current && !timeline) {
      const items = new DataSet<CustomTimelineItem>([]);
      const groups = new DataSet([
        { id: 0, content: 'Partition 0' },
        { id: 1, content: 'Partition 1' },
        { id: 2, content: 'Partition 2' }
      ]);

      const options: TimelineOptions = {
        height: '400px',
        cluster: {
          maxItems: 5,
          clusterCriteria: (firstItem: any, secondItem: any) => {
            const item1 = firstItem as CustomTimelineItem;
            const item2 = secondItem as CustomTimelineItem;
            const diff = Math.abs(item1.start.getTime() - item2.start.getTime());
            return diff < 1000 * 60 * 5; // Cluster items within 5 minutes
          }
        },
        tooltip: {
          followMouse: true,
          overflowMethod: 'cap'
        }
      };

      const newTimeline = new Timeline(timelineRef.current, items, groups, options);
      setTimeline(newTimeline);
    }
  }, [timelineRef.current]);

  useEffect(() => {
    const fetchMessages = async () => {
      try {
        const result = await getMessages('default-topic', limit);
        setMessages(result.messages);
        if (timeline) {
          const items = createTimelineItems(result.messages);
          timeline.setItems(new DataSet(items));
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch messages');
      }
    };

    fetchMessages();
    const interval = setInterval(fetchMessages, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, [timeline, limit]);

  const handleZoom = (factor: number) => {
    if (timeline) {
      const currentWindow = timeline.getWindow();
      const interval = currentWindow.end.getTime() - currentWindow.start.getTime();
      const newInterval = interval * factor;
      
      const center = (currentWindow.end.getTime() + currentWindow.start.getTime()) / 2;
      const newStart = new Date(center - newInterval / 2);
      const newEnd = new Date(center + newInterval / 2);
      
      timeline.setWindow(newStart, newEnd);
    }
  };

  return (
    <Box sx={{ mt: 4 }}>
      <Paper sx={{ p: 2 }}>
        <Typography variant="h6" gutterBottom>
          Message Timeline
        </Typography>
        
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <Typography sx={{ mr: 2 }}>
            Message Limit: {limit}
          </Typography>
          <Slider
            value={limit}
            onChange={(_, value) => setLimit(value as number)}
            min={10}
            max={100}
            step={10}
            sx={{ width: 200, mr: 2 }}
          />
          <Tooltip title="Zoom In">
            <IconButton onClick={() => handleZoom(0.7)}>
              <ZoomIn />
            </IconButton>
          </Tooltip>
          <Tooltip title="Zoom Out">
            <IconButton onClick={() => handleZoom(1.3)}>
              <ZoomOut />
            </IconButton>
          </Tooltip>
        </Box>

        <div ref={timelineRef} />

        {error && (
          <Typography color="error" sx={{ mt: 2 }}>
            {error}
          </Typography>
        )}
      </Paper>
    </Box>
  );
};

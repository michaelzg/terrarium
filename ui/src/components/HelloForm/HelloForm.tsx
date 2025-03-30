import { useState } from 'react';
import { TextField, Button, Box, Alert, CircularProgress } from '@mui/material';
import { sayHello } from '../../services/grpc/client';

export const HelloForm = () => {
  const [name, setName] = useState('world');
  const [loading, setLoading] = useState(false);
  const [response, setResponse] = useState('');
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    setResponse('');

    try {
      const result = await sayHello(name);
      setResponse(result.message);
    } catch (err) {
      console.error('gRPC error:', err);
      setError(err instanceof Error ? err.message : 'Failed to send message');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box component="form" onSubmit={handleSubmit} sx={{ maxWidth: 400, margin: '0 auto', mt: 4 }}>
      <TextField
        fullWidth
        label="Name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        margin="normal"
        disabled={loading}
      />
      
      <Button 
        type="submit"
        variant="contained"
        fullWidth
        disabled={loading}
        sx={{ mt: 2 }}
      >
        {loading ? <CircularProgress size={24} /> : 'Say Hello'}
      </Button>

      {response && (
        <Alert severity="success" sx={{ mt: 2 }}>
          {response}
        </Alert>
      )}

      {error && (
        <Alert severity="error" sx={{ mt: 2 }}>
          {error}
        </Alert>
      )}
    </Box>
  );
};

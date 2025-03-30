import { Container, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import { HelloForm } from './components/HelloForm/HelloForm';
import { MessageTimeline } from './components/MessageTimeline/MessageTimeline';

const theme = createTheme({
  palette: {
    mode: 'light',
    primary: {
      main: '#1976d2',
    },
    secondary: {
      main: '#dc004e',
    },
  },
});

function App() {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Container>
        <HelloForm />
        <MessageTimeline />
      </Container>
    </ThemeProvider>
  );
}

export default App;

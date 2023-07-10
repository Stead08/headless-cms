// import jsonServerProvider from 'ra-data-json-server';
import { Route, Routes } from 'react-router-dom';
import Home from './routes/home.tsx';
import Dashboard from './routes/dashboard.tsx';
import Login from './routes/login.tsx';
import Logout from './routes/logout.tsx';
// const dataProvider = jsonServerProvider('https://jsonplaceholder.typicode.com');

const App = () => {
  return (
    <div className="App">
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/login" element={<Login />} />
        <Route path="/logout" element={<Logout />} />
        <Route path="/dashboard" element={<Dashboard />} />
      </Routes>
    </div>
  );
};

export default App;

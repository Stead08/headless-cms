import { Link } from 'react-router-dom';
import Button from '../../../common-ui/dist/Button.svelte';
import { useEffect, useRef } from 'react';
import Login from '../components/Login.tsx';
import Logout from '../components/Logout.tsx';

const Home = () => {
  const containerRef = useRef(null);

  useEffect(() => {
    if (containerRef.current) {
      const button = new Button({
        target: containerRef.current,
        props: {
          label: 'this is svelte component',
          backgroundColor: 'orange',
        },
      });
      return () => button.$destroy();
    }
  });
  return (
    <div>
      <h1>Home</h1>
      <h3>React + Vite</h3>
      <ul>
        <li>
          <Link to="/dashboard">Dashboard</Link>
        </li>
        <li>
          <Login />
        </li>
        <li>
          <Logout />
        </li>
      </ul>
    </div>
  );
};

export default Home;

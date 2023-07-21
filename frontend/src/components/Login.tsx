import { useAuth0 } from '@auth0/auth0-react';
import Button from '../../../common-ui/dist/Button.svelte';
import { useEffect, useRef } from 'react';

const Login = () => {
  const { loginWithRedirect } = useAuth0();
  const containerRef = useRef(null);

  useEffect(() => {
    if (containerRef.current) {
      const button = new Button({
        target: containerRef.current,
        props: {
          primary: false,
          label: 'Login',
        },
      });
      return () => button.$destroy();
    }
  });
  return (
    <div
      ref={containerRef}
      onClick={() => {
        loginWithRedirect().catch((e) => console.error(e));
      }}
    ></div>
  );
};

export default Login;

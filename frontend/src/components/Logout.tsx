import Button from '../../../common-ui/dist/Button.svelte';
import { useAuth0 } from '@auth0/auth0-react';
import { useEffect, useRef } from 'react';

const Logout = () => {
  const { logout } = useAuth0();
  const containerRef = useRef(null);

  useEffect(() => {
    if (containerRef.current) {
      const button = new Button({
        target: containerRef.current,
        props: {
          primary: false,
          label: 'Logout',
        },
      });
      return () => button.$destroy();
    }
  });
  return (
    <div
      ref={containerRef}
      onClick={() =>
        logout({ logoutParams: { returnTo: window.location.origin } })
      }
    ></div>
  );
};

export default Logout;

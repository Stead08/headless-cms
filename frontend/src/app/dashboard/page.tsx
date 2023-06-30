"use client";

import { withPageAuthRequired } from "@auth0/nextjs-auth0/client";
import { useState } from "react";
import { Button } from "@/stories/Button";

export default withPageAuthRequired(function Profile({ user }) {
  const [profile, setProfile] = useState(null);
  const fetch_profile = async (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    const response = await fetch("../api/dashboard");
    const data = await response.json();
    console.log(data);
    setProfile(data);
  };
  return (
    <>
      <div>
        {user ? <><Button label={"情報を取得"} onClick={(e: React.MouseEvent<HTMLButtonElement>) => fetch_profile(e)} />
            {JSON.stringify(profile)}
         </> : <p> ログインしてください </p>}
      </div>
    </>
  );
});

